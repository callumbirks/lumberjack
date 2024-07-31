use core::fmt;
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read, Seek},
    path::Path,
};

use chrono::{DateTime, NaiveDateTime, TimeDelta};

use crate::{Error, Result};

const MAGIC_NUMBER: [u8; 4] = [0xCF, 0xB2, 0xAB, 0x1B];
const FORMAT_VERSION: u8 = 1;
const LEVEL_NAMES: [&str; 5] = ["Debug", "Verbose", "Info", "Warning", "Error"];
const TS_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.6f";
const TICKS_PER_SECOND: u64 = 1_000_000;

pub fn is_encoded(path: &Path) -> Result<bool> {
    let mut file = std::fs::File::open(path)?;
    let mut magic = [0_u8; 4];
    file.read_exact(&mut magic)?;
    Ok(magic == MAGIC_NUMBER)
}

pub fn decode_lines(path: &Path) -> Result<Vec<String>> {
    let file = std::fs::File::open(path)?;
    let buf_reader = BufReader::new(file);
    let mut decoder = Decoder::new(buf_reader)?;
    decoder.lines().collect()
}

struct Decoder<R>
where
    R: BufRead + Seek,
{
    reader: R,
    pointer_size: u8,
    start_time: NaiveDateTime,
    elapsed_ticks: u64,
    tokens: Vec<String>,
    objects: BTreeMap<u64, String>,
}

impl<R> Decoder<R>
where
    R: BufRead + Seek,
{
    fn new(mut reader: R) -> Result<Self> {
        let mut header = [0_u8; 6];
        reader.read_exact(&mut header)?;
        if &header[..4] != &MAGIC_NUMBER {
            return Err(Error::InvalidBinaryLogs("Invalid header".into(), 0));
        }
        if header[4] != FORMAT_VERSION {
            return Err(Error::InvalidBinaryLogs(
                format!(
                    "Unsupported format version '{}', expected '{}'",
                    header[4], FORMAT_VERSION
                ),
                0,
            ));
        }
        let pointer_size: u8 = header[5];
        if pointer_size != 4 && pointer_size != 8 {
            return Err(Error::InvalidBinaryLogs("Invalid header".into(), 0));
        }
        let Some(start_time) = DateTime::from_timestamp(varint::read(&mut reader)? as i64, 0)
        else {
            return Err(Error::InvalidBinaryLogs(
                "Invalid timestamp in header".into(),
                0,
            ));
        };
        log::trace!(
            "Started Decoder with start time: {}, pointer size: {}",
            start_time.format(TS_FORMAT),
            pointer_size
        );
        Ok(Self {
            reader,
            pointer_size,
            start_time: start_time.naive_utc(),
            elapsed_ticks: 0,
            tokens: vec![],
            objects: BTreeMap::new(),
        })
    }

    fn entries(&mut self) -> impl Iterator<Item = Result<DecoderEntry>> + '_ {
        std::iter::from_fn(move || match self.read_entry() {
            Ok(Some(entry)) => Some(Ok(entry)),
            Ok(None) => None,
            Err(err) => Some(Err(err)),
        })
    }

    fn lines(&mut self) -> impl Iterator<Item = Result<String>> + '_ {
        self.entries()
            .map(|entry| entry.map(|entry| entry.to_string()))
    }

    fn read_entry(&mut self) -> Result<Option<DecoderEntry>> {
        let Ok(timestamp) = self.read_timestamp() else {
            return Ok(None);
        };
        log::trace!("Read timestamp {}", timestamp.format(TS_FORMAT));
        let level = self.read_byte()? as i8;
        log::trace!("Read level {}", level);
        let level = if level > 0 {
            LEVEL_NAMES.get(level as usize).ok_or_else(|| {
                self.create_err(format!("No known log level with discriminant {}", level))
            })?
        } else {
            ""
        };
        let domain = self.read_tokenized_string()?.clone();
        log::trace!("Read domain '{}'", domain);
        let object = self.read_object()?;
        log::trace!("Read object '{}'", object.as_ref().unwrap_or(&"".into()));
        let message = self.read_message()?;
        log::trace!("Read message '{}'", message);
        Ok(Some(DecoderEntry {
            timestamp,
            domain,
            level,
            object,
            message,
        }))
    }

    fn read_message(&mut self) -> Result<String> {
        let format_chars: Vec<char> = {
            let format_string = self.read_tokenized_string()?;
            log::trace!("Read format string: '{}'", format_string);
            format_string.chars().collect()
        };
        let mut message = String::new();

        let mut i: usize = 0;

        while i < format_chars.len() {
            if format_chars[i] == '\0' {
                log::trace!("Read NULL terminator");
                break;
            }
            if format_chars[i] != '%' {
                message.push(format_chars[i]);
                i += 1;
                continue;
            }

            let is_minus = format_chars[i + 1] == '-';
            i = if is_minus { i + 2 } else { i + 1 };

            while "#0- +'".contains(format_chars[i]) {
                i += 1;
            }
            while format_chars[i].is_digit(10) {
                i += 1;
            }

            let is_dot_star = if format_chars[i] == '.' {
                i += 1;
                if format_chars[i] == '*' {
                    i += 1;
                    true
                } else {
                    while format_chars[i].is_digit(10) {
                        i += 1;
                    }
                    false
                }
            } else {
                false
            };

            while "hljtzq".contains(format_chars[i]) {
                i += 1;
            }

            let c = format_chars[i];
            match c {
                'c' | 'd' | 'i' => {
                    let is_negative = self.read_byte()? > 0;
                    let value: i64 = varint::read(&mut self.reader)?
                        .try_into()
                        .expect("Overflow in numeric parameter!");
                    let value = if is_negative { -value } else { value };
                    if c == 'c' {
                        message.push(value as u8 as char);
                    } else {
                        message.push_str(&value.to_string());
                    }
                }
                'x' | 'X' => {
                    let value = varint::read(&mut self.reader)?;
                    message.push_str(&format!("{:02x}", value));
                }
                'u' => {
                    let value = varint::read(&mut self.reader)?;
                    message.push_str(&value.to_string());
                }
                'e' | 'E' | 'f' | 'F' | 'g' | 'G' | 'a' | 'A' => {
                    let mut buf = [0_u8; 8];
                    self.reader.read_exact(&mut buf)?;
                    let value = f64::from_le_bytes(buf);
                    message.push_str(&value.to_string());
                }
                '@' | 's' if is_minus && !is_dot_star => {
                    let string = self.read_tokenized_string()?;
                    message.push_str(&string);
                }
                '@' | 's' => {
                    let length = varint::read(&mut self.reader)? as usize;
                    let mut string = "0".repeat(length);
                    self.reader.read_exact(unsafe { string.as_bytes_mut() })?;
                    if is_minus {
                        string = string
                            .into_bytes()
                            .into_iter()
                            .map(|b| format!("{:02x}", b))
                            .collect::<String>();
                    }
                    message.push_str(&string);
                }
                'p' if self.pointer_size == 8 => {
                    let mut buf = [0_u8; 8];
                    self.reader.read_exact(&mut buf)?;
                    let value = u64::from_le_bytes(buf);
                    message.push_str(&format!("{:#016x}", value));
                }
                'p' if self.pointer_size == 4 => {
                    let mut buf = [0_u8; 4];
                    self.reader.read_exact(&mut buf)?;
                    let value = u32::from_le_bytes(buf);
                    message.push_str(&format!("{:#08x}", value));
                }
                '%' => {
                    message.push('%');
                }
                _ => {
                    return Err(self.create_err(format!("Unknown format specifier '{}'", c)));
                }
            }
            i += 1;
        }

        Ok(message)
    }

    fn read_timestamp(&mut self) -> Result<NaiveDateTime> {
        self.elapsed_ticks += varint::read(&mut self.reader).map_err(|err| self.map_err(err))?;
        let timestamp = self.start_time
            + TimeDelta::seconds(
                (self.elapsed_ticks / TICKS_PER_SECOND)
                    .try_into()
                    .expect("Overflow in timestamp calculation!"),
            )
            + TimeDelta::microseconds(
                (self.elapsed_ticks % TICKS_PER_SECOND)
                    .try_into()
                    .expect("Overflow in timestamp calculation!"),
            );
        Ok(timestamp)
    }

    fn read_byte(&mut self) -> Result<u8> {
        self.reader.read_byte().map_err(|err| self.map_err(err))
    }

    fn read_object(&mut self) -> Result<Option<String>> {
        let object_id = varint::read(&mut self.reader).map_err(|err| self.map_err(err))?;
        if object_id == 0 {
            Ok(None)
        } else if let Some(object) = self.objects.get(&object_id) {
            Ok(Some(object.clone()))
        } else {
            let object = self.read_string()?;
            self.objects.insert(object_id, object.clone());
            Ok(Some(object))
        }
    }

    fn read_tokenized_string(&mut self) -> Result<&String> {
        let token_id = varint::read(&mut self.reader).map_err(|err| self.map_err(err))? as usize;
        if token_id < self.tokens.len() {
            Ok(&self.tokens[token_id])
        } else if token_id == self.tokens.len() {
            let string = self.read_string()?;
            self.tokens.push(string);
            Ok(&self.tokens[token_id])
        } else {
            Err(self.create_err("Invalid token string ID!".into()))
        }
    }

    /// Read a null-terminated string from the reader.
    fn read_string(&mut self) -> Result<String> {
        let mut string = String::with_capacity(20);
        loop {
            let byte = self.read_byte().map_err(|err| self.map_err(err))?;
            if byte == 0 {
                break;
            }
            string.push(byte as char);
        }
        Ok(string)
    }

    #[inline]
    fn create_err(&mut self, msg: String) -> Error {
        let err = Error::InvalidBinaryLogs(msg, self.reader.stream_position().unwrap_or(0));
        // UNCOMMENT FOR DEBUG PURPOSES!
        //if log::log_enabled!(log::Level::Trace) {
        //    let backtrace = std::backtrace::Backtrace::capture();
        //    if backtrace.status() != BacktraceStatus::Captured {
        //        log::error!("{}\n\tBacktrace is disabled. Try setting `RUST_BACKTRACE=1` if you want backtraces for this error.", err);
        //    } else {
        //        log::error!("{}\n{}", err, backtrace);
        //    }
        //}
        err
    }

    #[inline]
    fn map_err<E: std::error::Error>(&mut self, error: E) -> Error {
        self.create_err(error.to_string())
    }
}

struct DecoderEntry {
    timestamp: NaiveDateTime,
    domain: String,
    level: &'static str,
    object: Option<String>,
    message: String,
}

impl fmt::Display for DecoderEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let timestamp = self.timestamp.format(TS_FORMAT);
        if let Some(object) = self.object.as_deref() {
            write!(
                f,
                "{} {} {} Obj={} {}",
                timestamp, self.domain, self.level, object, self.message
            )
        } else {
            write!(
                f,
                "{} {} {} {}",
                timestamp, self.domain, self.level, self.message
            )
        }
    }
}

trait ReadByte: Read {
    #[inline]
    fn read_byte(&mut self) -> std::io::Result<u8> {
        let mut byte = [0_u8];
        self.read_exact(&mut byte)?;
        Ok(byte[0])
    }
}

impl<T: Read> ReadByte for T {}

mod varint {
    use crate::decoder::ReadByte;
    use crate::{Error, Result};
    use std::io::Read;

    const MAX_LEN: usize = 10;

    pub fn read<R>(reader: &mut R) -> Result<u64>
    where
        R: Read,
    {
        let mut res: u64 = 0;

        for i in 0..MAX_LEN {
            let byte = reader.read_byte()?;
            res |= u64::from(byte & 0x7F) << 7 * i;
            if byte < 0x80 {
                return Ok(res);
            }
        }

        Err(Error::InvalidVarint)
    }
}
