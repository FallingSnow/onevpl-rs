use std::{mem, io};

use ffi::mfxBitstream;
use intel_onevpl_sys as ffi;

use crate::constants::{Codec, BitstreamDataFlags};

#[derive(Debug)]
pub struct Bitstream<'a> {
    buffer: &'a mut [u8],
    pub(crate) inner: mfxBitstream,
}

impl<'a> Bitstream<'a> {
    /// Creates a data source/destination for encoded/decoded/processed data
    #[tracing::instrument]
    pub fn with_codec(buffer: &'a mut [u8], codec: Codec) -> Self {
        let mut bitstream: mfxBitstream = unsafe { mem::zeroed() };
        bitstream.Data = buffer.as_mut_ptr();
        bitstream.MaxLength = buffer.len() as u32;
        bitstream.__bindgen_anon_1.__bindgen_anon_1.CodecId = codec as u32;
        Self {
            buffer,
            inner: bitstream,
        }
    }

    pub fn codec(&self) -> Codec {
        Codec::from_repr(unsafe { self.inner.__bindgen_anon_1.__bindgen_anon_1.CodecId }).unwrap()
    }

    /// The size of the backing buffer
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// The amount of data currently in the bitstream
    pub fn size(&self) -> u32 {
        self.inner.DataLength
    }

    pub fn set_flags(&mut self, flags: BitstreamDataFlags) {
        self.inner.DataFlag = flags.bits();
    }
}

impl io::Write for Bitstream<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let data_offset = self.inner.DataOffset as usize;
        let data_len = self.inner.DataLength as usize;

        let slice = &mut self.buffer;

        if data_len >= slice.len() {
            return Ok(0);
        }

        if data_offset > 0 {
            // Move all data after DataOffset to the beginning of Data
            let data_end = data_offset + data_len;
            slice.copy_within(data_offset..data_end, 0);
            self.inner.DataOffset = 0;
        }

        let free_buffer_len = slice.len() - data_len;
        let copy_len = usize::min(free_buffer_len, buf.len());
        slice[data_len..data_len + copy_len].copy_from_slice(&buf[..copy_len]);
        self.inner.DataLength += copy_len as u32;

        Ok(copy_len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
