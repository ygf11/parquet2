mod binary;
mod fixed_len_binary;
mod primitive;

pub use binary::BinaryPageDict;
pub use fixed_len_binary::FixedLenByteArrayPageDict;
pub use primitive::PrimitivePageDict;

use std::{any::Any, sync::Arc};

use crate::compression::{decompress, Compression};
use crate::error::{Error, Result};
use crate::schema::types::PhysicalType;

/// A dynamic trait describing a decompressed and decoded Dictionary Page.
pub trait DictPage: std::fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;

    fn physical_type(&self) -> &PhysicalType;
}

/// A encoded and uncompressed dictionary page.
#[derive(Debug)]
pub struct EncodedDictPage {
    pub(crate) buffer: Vec<u8>,
    pub(crate) num_values: usize,
}

impl EncodedDictPage {
    pub fn new(buffer: Vec<u8>, num_values: usize) -> Self {
        Self { buffer, num_values }
    }
}

/// An encoded and compressed dictionary page.
#[derive(Debug)]
pub struct CompressedDictPage {
    pub(crate) buffer: Vec<u8>,
    compression: Compression,
    pub(crate) num_values: usize,
    pub(crate) uncompressed_page_size: usize,
}

impl CompressedDictPage {
    pub fn new(buffer: Vec<u8>, compression: Compression, uncompressed_page_size: usize, num_values: usize) -> Self {
        Self {
            buffer,
            compression,
            uncompressed_page_size,
            num_values,
        }
    }

    /// The compression of the data in this page.
    pub fn compression(&self) -> Compression {
        self.compression
    }
}

pub fn read_dict_page(
    page: &EncodedDictPage,
    compression: (Compression, usize),
    is_sorted: bool,
    physical_type: PhysicalType,
) -> Result<Arc<dyn DictPage>> {
    if compression.0 != Compression::Uncompressed {
        let mut decompressed = vec![0; compression.1];
        decompress(compression.0, &page.buffer, &mut decompressed)?;
        deserialize(&decompressed, page.num_values, is_sorted, physical_type)
    } else {
        deserialize(&page.buffer, page.num_values, is_sorted, physical_type)
    }
}

fn deserialize(
    buf: &[u8],
    num_values: usize,
    is_sorted: bool,
    physical_type: PhysicalType,
) -> Result<Arc<dyn DictPage>> {
    match physical_type {
        PhysicalType::Boolean => Err(Error::OutOfSpec(
            "Boolean physical type cannot be dictionary-encoded".to_string(),
        )),
        PhysicalType::Int32 => primitive::read::<i32>(buf, num_values, is_sorted),
        PhysicalType::Int64 => primitive::read::<i64>(buf, num_values, is_sorted),
        PhysicalType::Int96 => primitive::read::<[u32; 3]>(buf, num_values, is_sorted),
        PhysicalType::Float => primitive::read::<f32>(buf, num_values, is_sorted),
        PhysicalType::Double => primitive::read::<f64>(buf, num_values, is_sorted),
        PhysicalType::ByteArray => binary::read(buf, num_values),
        PhysicalType::FixedLenByteArray(size) => fixed_len_binary::read(buf, size, num_values),
    }
}
