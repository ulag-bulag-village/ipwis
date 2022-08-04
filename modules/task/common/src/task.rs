use std::collections::HashMap;

use bytecheck::CheckBytes;
use ipis::{
    core::{account::GuarantorSigned, data::Data, signed::IsSigned},
    path::Path,
};
use rkyv::{Archive, Deserialize, Serialize};

use crate::task_constraints::TaskConstraints;

#[derive(Clone, Debug, PartialEq, Archive, Serialize, Deserialize)]
#[archive(bound(serialize = "
    __S: ::rkyv::ser::ScratchSpace + ::rkyv::ser::Serializer,
"))]
#[archive_attr(derive(Debug, PartialEq))]
pub struct Task {
    pub constraints: TaskConstraints,
    pub program: Option<Data<GuarantorSigned, Path>>,
    #[omit_bounds]
    pub reserved: HashMap<String, Self>,
    #[omit_bounds]
    pub children: HashMap<String, Self>,
    #[omit_bounds]
    pub exceptions: Vec<Self>,
}

impl Task {
    pub fn new_sandbox() -> Self {
        Self {
            constraints: TaskConstraints::new_sandbox(),
            program: None,
            reserved: Default::default(),
            children: Default::default(),
            exceptions: Default::default(),
        }
    }
}

impl IsSigned for Task {}

impl<__C> CheckBytes<__C> for ArchivedTask
where
    __C: ::rkyv::validation::ArchiveContext,
    <__C as ::rkyv::Fallible>::Error: ::std::error::Error,
{
    type Error = ::bytecheck::StructCheckError;

    unsafe fn check_bytes<'__bytecheck>(
        value: *const Self,
        context: &mut __C,
    ) -> Result<&'__bytecheck Self, Self::Error> {
        CheckBytes::<__C>::check_bytes(::core::ptr::addr_of!((*value).constraints), context)
            .map_err(|e| ::bytecheck::StructCheckError {
                field_name: stringify!(constraints),
                inner: ::bytecheck::ErrorBox::new(e),
            })?;
        CheckBytes::<__C>::check_bytes(::core::ptr::addr_of!((*value).program), context).map_err(
            |e| ::bytecheck::StructCheckError {
                field_name: stringify!(program),
                inner: ::bytecheck::ErrorBox::new(e),
            },
        )?;
        CheckBytes::<__C>::check_bytes(::core::ptr::addr_of!((*value).reserved), context).map_err(
            |e| ::bytecheck::StructCheckError {
                field_name: stringify!(reserved),
                inner: ::bytecheck::ErrorBox::new(e),
            },
        )?;
        CheckBytes::<__C>::check_bytes(::core::ptr::addr_of!((*value).children), context).map_err(
            |e| ::bytecheck::StructCheckError {
                field_name: stringify!(children),
                inner: ::bytecheck::ErrorBox::new(e),
            },
        )?;
        CheckBytes::<__C>::check_bytes(::core::ptr::addr_of!((*value).exceptions), context)
            .map_err(|e| ::bytecheck::StructCheckError {
                field_name: stringify!(exceptions),
                inner: ::bytecheck::ErrorBox::new(e),
            })?;
        Ok(&*value)
    }
}
