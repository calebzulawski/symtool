use crate::error::{Error, Result};

pub(crate) fn get_mutable(data: &mut [u8], source: *const u8, len: usize) -> &mut [u8] {
    let data_offset = data.as_ptr() as usize;
    let src_offset = source as usize;
    assert!(src_offset > data_offset);
    let begin = src_offset - data_offset;
    let end = begin + len;
    &mut data[begin..end]
}

pub(crate) fn replace_str(data: &mut [u8], source: &str, replacement: &str) -> Result<()> {
    if replacement.len() > source.len() {
        Err(Error::ReplaceString {
            original: source.to_string(),
            replacement: replacement.to_string(),
        })
    } else {
        get_mutable(data, source.as_ptr(), source.len())[0..replacement.len()]
            .copy_from_slice(replacement.as_bytes());
        Ok(())
    }
}
