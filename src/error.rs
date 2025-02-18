//
// Copyright (c) 2016 KAMADA Ken'ichi.
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
// 1. Redistributions of source code must retain the above copyright
//    notice, this list of conditions and the following disclaimer.
// 2. Redistributions in binary form must reproduce the above copyright
//    notice, this list of conditions and the following disclaimer in the
//    documentation and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
// ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
// OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
// HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
// LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
// OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
// SUCH DAMAGE.
//

use std::error;
use std::fmt;
use std::io;
use std::sync::Mutex;

/// An error returned when parsing of Exif data fails.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Input data was malformed or truncated.
    InvalidFormat(&'static str),
    /// Input data could not be read due to an I/O error and
    /// a `std::io::Error` value is associated with this variant.
    Io(io::Error),
    /// Exif attribute information was not found in an image file
    /// such as JPEG.
    NotFound(&'static str),
    /// The value of the field is blank.  Some fields have blank values
    /// whose meanings are defined as "unknown".  Such a blank value
    /// should be treated the same as the absence of the field.
    BlankValue(&'static str),
    /// Field values or image data are too big to encode.
    TooBig(&'static str),
    /// The field type is not supported and cannnot be encoded.
    NotSupported(&'static str),
    /// The field has an unexpected value.
    UnexpectedValue(&'static str),
    /// Partially-parsed result and errors.  This can be returned only when
    /// `Reader::continue_on_error` is enabled.
    PartialResult(PartialResult),
}

impl Error {
    /// Extracts `Exif` and `Vec<Error>` from `Error::PartialResult`.
    ///
    /// If `self` is `Error::PartialResult`,
    /// ignored errors are passed to `f` as `Vec<Error>` and
    /// partially-parsed result is retuend in `Ok`.
    /// Otherwise, `Err(self)` is returned.
    pub fn distill_partial_result<F>(self, f: F) -> Result<crate::Exif, Self>
    where F: FnOnce(Vec<Error>) {
        if let Error::PartialResult(partial) = self {
            let (exif, errors) = partial.into_inner();
            f(errors);
            Ok(exif)
        } else {
            Err(self)
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::InvalidFormat(msg) => f.write_str(msg),
            Error::Io(ref err) => err.fmt(f),
            Error::NotFound(ctn) => write!(f, "No Exif data found in {}", ctn),
            Error::BlankValue(msg) => f.write_str(msg),
            Error::TooBig(msg) => f.write_str(msg),
            Error::NotSupported(msg) => f.write_str(msg),
            Error::UnexpectedValue(msg) => f.write_str(msg),
            Error::PartialResult(ref pr) =>
                write!(f, "Partial result with {} fields and {} errors",
                       pr.0.0.lock().expect("should not panic").fields().len(),
                       pr.0.1.len()),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::InvalidFormat(_) => None,
            Error::Io(ref err) => Some(err),
            Error::NotFound(_) => None,
            Error::BlankValue(_) => None,
            Error::TooBig(_) => None,
            Error::NotSupported(_) => None,
            Error::UnexpectedValue(_) => None,
            Error::PartialResult(_) => None,
        }
    }
}

/// Partially-parsed result and errors.
pub struct PartialResult(Box<(Mutex<crate::Exif>, Vec<Error>)>);

impl PartialResult {
    pub(crate) fn new(exif: crate::Exif, errors: Vec<Error>) -> Self {
        Self(Box::new((Mutex::new(exif), errors)))
    }

    /// Returns partially-parsed `Exif` and ignored `Error`s.
    pub fn into_inner(self) -> (crate::Exif, Vec<Error>) {
        let (exif, errors) = *self.0;
        (exif.into_inner().expect("should not panic"), errors)
    }
}

impl fmt::Debug for PartialResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PartialResult(Exif({} fields), {:?})",
               self.0.0.lock().expect("should not panic").fields().len(),
               self.0.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Check compatibility with anyhow::Error, which requires Send, Sync,
    // and 'static on error types.
    #[test]
    fn is_send_sync_static() {
        let _: Box<dyn Send + Sync + 'static> =
            Box::new(Error::InvalidFormat("test"));
    }
}
