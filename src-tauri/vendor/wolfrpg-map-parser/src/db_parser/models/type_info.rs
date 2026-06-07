use crate::byte_utils::{as_blob, as_u32_le, as_u32_vec, parse_string, parse_string_vec};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Information on a database type.
///
/// By database type, we mean a table, containing several fields and entries.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct TypeInfo {
    index: usize,
    name: String,
    fields: Vec<TypeField>,
    data_names: Vec<String>,
    note: String,
}

impl TypeInfo {
    pub(crate) fn parse(bytes: &[u8], index: usize) -> (usize, Self) {
        let mut offset: usize = 0;

        let (bytes_read, name): (usize, String) = parse_string(&bytes[offset..]);
        offset += bytes_read;

        let field_count: usize = as_u32_le(&bytes[offset..]) as usize;
        offset += 4;

        let (bytes_read, fields): (usize, Vec<String>)
            = parse_string_vec(&bytes[offset..], field_count);
        offset += bytes_read;

        let data_count: usize = as_u32_le(&bytes[offset..]) as usize;
        offset += 4;

        let (bytes_read, data_names): (usize, Vec<String>)
            = parse_string_vec(&bytes[offset..], data_count);
        offset += bytes_read;

        let (bytes_read, note): (usize, String) = parse_string(&bytes[offset..]);
        offset += bytes_read;

        let (bytes_read, field_data_types): (usize, Vec<u8>)
            = as_blob(&bytes[offset..], 1);
        offset += bytes_read;

        let (bytes_read, _) = as_blob(&bytes[offset..], 5);
        offset += bytes_read;

        let _ = as_u32_le(&bytes[offset..]);
        offset += 4;

        let mut field_strings: Vec<Vec<String>> = Vec::with_capacity(field_count);

        for _ in 0..field_count {
            let string_count: usize = as_u32_le(&bytes[offset..]) as usize;
            offset += 4;

            let (bytes_read, strings): (usize, Vec<String>)
                = parse_string_vec(&bytes[offset..], string_count);
            offset += bytes_read;

            field_strings.push(strings);
        }

        let _ = as_u32_le(&bytes[offset..]);
        offset += 4;

        let mut field_metas: Vec<Vec<i32>> = Vec::with_capacity(field_count);

        for _ in 0..field_count {
            let count: usize = as_u32_le(&bytes[offset..]) as usize;
            offset += 4;

            let field_meta: Vec<i32> = as_u32_vec(&bytes[offset..][..4 * count])
                .iter().map(|u| *u as i32)
                .collect();
            offset += 4 * count;

            field_metas.push(field_meta);
        }

        let _ = as_u32_le(&bytes[offset..]);
        offset += 4;

        let default_values: Vec<i32> = as_u32_vec(&bytes[offset..][.. 4 * field_count])
            .iter().map(|u| *u as i32)
            .collect();
        offset += 4 * field_count;

        let fields: Vec<TypeField> = fields.iter().enumerate().map(|(i, str)| {
            TypeField {
                name: str.clone(),
                category: field_data_types[i],
                strings: field_strings[i].clone(),
                meta: field_metas[i].clone(),
                default: default_values[i]
            }
        }).collect();

        (offset, Self {
            index,
            name,
            fields,
            data_names,
            note
        })
    }

    /// The index of this type in the database schema.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The name given to this database type.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Mutable reference accessor for [`TypeInfo::name`].
    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    /// A list of fields for this database type.
    pub fn fields(&self) -> &Vec<TypeField> {
        &self.fields
    }

    /// Mutable reference accessor for [`TypeInfo::fields`].
    pub fn fields_mut(&mut self) -> &mut Vec<TypeField> {
        &mut self.fields
    }

    /// A list of names associated with each entry.
    pub fn data_names(&self) -> &Vec<String> {
        &self.data_names
    }

    /// Mutable reference accessor for [`TypeInfo::data_names`].
    pub fn data_names_mut(&mut self) -> &mut Vec<String> {
        &mut self.data_names
    }

    /// A brief description of this database type.
    pub fn note(&self) -> &str {
        &self.note
    }

    /// Mutable reference accessor for [`TypeInfo::note`].
    pub fn note_mut(&mut self) -> &mut String {
        &mut self.note
    }
}

/// Detailed information regarding a field of the table.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct TypeField {
    name: String,
    category: u8,
    strings: Vec<String>,
    meta: Vec<i32>,
    default: i32
}

impl TypeField {
    /// The name of the field.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Mutable reference accessor for [`TypeField::name`].
    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    /// The field type/category.
    pub fn category(&self) -> u8 {
        self.category
    }

    /// Mutable reference accessor for [`TypeField::category`].
    pub fn special_mut(&mut self) -> &mut u8 {
        &mut self.category
    }

    /// A list of possible string values for this field.
    pub fn strings(&self) -> &Vec<String> {
        &self.strings
    }

    /// Mutable reference accessor for [`TypeField::strings`].
    pub fn strings_mut(&mut self) -> &mut Vec<String> {
        &mut self.strings
    }

    /// Metadata for this field.
    pub fn meta(&self) -> &Vec<i32> {
        &self.meta
    }

    /// Mutable reference accessor for [`TypeField::meta`].
    pub fn meta_mut(&mut self) -> &mut Vec<i32> {
        &mut self.meta
    }

    /// Default value for this field.
    pub fn default(&self) -> i32 {
        self.default
    }

    /// Mutable reference accessor for [`TypeField::default`].
    pub fn default_mut(&mut self) -> &mut i32 {
        &mut self.default
    }
}