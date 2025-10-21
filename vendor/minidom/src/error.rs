// Copyright (c) 2020 lumi <lumi@pew.im>
// Copyright (c) 2020 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
// Copyright (c) 2020 Bastien Orivel <eijebong+minidom@bananium.fr>
// Copyright (c) 2020 Astro <astro@spaceboyz.net>
// Copyright (c) 2020 Maxime “pep” Buquet <pep@bouah.net>
// Copyright (c) 2020 Matt Bilker <me@mbilker.us>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Provides an error type for this crate.

use std::convert::From;
use std::error::Error as StdError;

/// Our main error type.
#[derive(Debug)]
pub enum Error {
    /// An error from quick_xml.
    XmlError(::quick_xml::Error),

    /// An UTF-8 conversion error.
    Utf8Error(::std::str::Utf8Error),

    /// An I/O error, from std::io.
    IoError(::std::io::Error),

    /// An error which is returned when the end of the document was reached prematurely.
    EndOfDocument,

    /// An error which is returned when an element is closed when it shouldn't be
    InvalidElementClosed,

    /// An error which is returned when an elemet's name contains more than one colon
    InvalidElement,

    /// An error which is returned when a comment is to be parsed by minidom
    #[cfg(not(comments))]
    CommentsDisabled,
}

impl StdError for Error {
    fn cause(&self) -> Option<&dyn StdError> {
        match self {
            // TODO: return Some(e) for this case after the merge of
            // https://github.com/tafia/quick-xml/pull/170
            Error::XmlError(_e) => None,
            Error::Utf8Error(e) => Some(e),
            Error::IoError(e) => Some(e),
            Error::EndOfDocument => None,
            Error::InvalidElementClosed => None,
            Error::InvalidElement => None,
            #[cfg(not(comments))]
            Error::CommentsDisabled => None,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::XmlError(e) => write!(fmt, "XML error: {}", e),
            Error::Utf8Error(e) => write!(fmt, "UTF-8 error: {}", e),
            Error::IoError(e) => write!(fmt, "IO error: {}", e),
            Error::EndOfDocument => {
                write!(fmt, "the end of the document has been reached prematurely")
            }
            Error::InvalidElementClosed => {
                write!(fmt, "the XML is invalid, an element was wrongly closed")
            }
            Error::InvalidElement => write!(fmt, "the XML element is invalid"),
            #[cfg(not(comments))]
            Error::CommentsDisabled => write!(
                fmt,
                "a comment has been found even though comments are disabled by feature"
            ),
        }
    }
}

impl From<::quick_xml::Error> for Error {
    fn from(err: ::quick_xml::Error) -> Error {
        Error::XmlError(err)
    }
}

impl From<::std::str::Utf8Error> for Error {
    fn from(err: ::std::str::Utf8Error) -> Error {
        Error::Utf8Error(err)
    }
}

impl From<::std::io::Error> for Error {
    fn from(err: ::std::io::Error) -> Error {
        Error::IoError(err)
    }
}

/// Our simplified Result type.
pub type Result<T> = ::std::result::Result<T, Error>;
