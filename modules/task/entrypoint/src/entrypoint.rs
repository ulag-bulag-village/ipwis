use proc_macro2::TokenStream;

pub fn expand_attribute(input: syn::ItemFn) -> Result<TokenStream, Vec<syn::Error>> {
    let syn::ItemFn {
        sig: syn::Signature { .. },
        block,
        ..
    } = input;

    // TODO: test fn signatures

    Ok(quote! {
        #[allow(clippy::missing_safety_doc)]
        #[cfg(target_os = "wasi")]
        mod memory {
            use std::alloc;

            #[no_mangle]
            pub unsafe extern "C" fn __ipwis_alloc(size: usize, align: usize) -> *mut u8 {
                alloc::alloc(alloc::Layout::from_size_align_unchecked(size, align))
            }

            #[no_mangle]
            pub unsafe extern "C" fn __ipwis_alloc_zeroed(size: usize, align: usize) -> *mut u8 {
                alloc::alloc_zeroed(alloc::Layout::from_size_align_unchecked(size, align))
            }

            #[no_mangle]
            pub unsafe extern "C" fn __ipwis_dealloc(ptr: *mut u8, size: usize, align: usize) {
                alloc::dealloc(ptr, alloc::Layout::from_size_align_unchecked(size, align))
            }

            #[no_mangle]
            pub unsafe extern "C" fn __ipwis_realloc(
                ptr: *mut u8,
                size: usize,
                align: usize,
                new_size: usize,
            ) -> *mut u8 {
                alloc::realloc(
                    ptr,
                    alloc::Layout::from_size_align_unchecked(size, align),
                    new_size,
                )
            }
        }

        #[allow(clippy::missing_safety_doc)]
        #[cfg(target_os = "wasi")]
        mod syscall {
            use ipis::{core::signed::IsSigned, object::data::ObjectData, pin::PinnedInner};
            use ipwis_modules_task_common_wasi::{
                extern_data::{ExternData, ExternDataRef},
                extrinsics::syscall,
            };

            #[no_mangle]
            unsafe extern "C" fn __ipwis_syscall(
                _handler: ExternDataRef,
                inputs: ExternDataRef,
                outputs: ExternDataRef,
                errors: ExternDataRef,
            ) -> ExternDataRef {
                let inputs = inputs as *const ExternData;
                let outputs = outputs as *mut ExternData;
                let errors = errors as *mut ExternData;

                let inputs: ObjectData = PinnedInner::deserialize_owned((*inputs).into_slice()).unwrap();

                let (buf, target, status_code) = match __ipwis_main(inputs) {
                    Ok(data) => {
                        let buf = Box::leak(data.to_bytes().unwrap().into_boxed_slice());
                        (buf, &mut *outputs, syscall::SYSCALL_OK)
                    }
                    Err(data) => {
                        let buf = Box::leak(data.to_string().into_bytes().into_boxed_slice());
                        (buf, &mut *errors, syscall::SYSCALL_ERR_NORMAL)
                    }
                };

                target.ptr = buf.as_ptr() as ExternDataRef;
                target.len = buf.len() as ExternDataRef;
                status_code
            }

            fn __ipwis_main(inputs: ObjectData) -> ::ipis::core::anyhow::Result<ObjectData> {
                ::ipis::futures::executor::block_on(__ipwis_main_async(inputs))
            }

            async fn __ipwis_main_async(inputs: ObjectData) -> ::ipis::core::anyhow::Result<ObjectData> {
                use super::*;

                #block
            }
        }

        #[cfg(not(target_os = "wasi"))]
        // TODO: use tokio::Runtime instead
        use ipis::tokio;

        #[cfg(not(target_os = "wasi"))]
        async fn __ipwis_main_async(inputs: ::ipis::object::data::ObjectData)
            -> ::ipis::core::anyhow::Result<::ipis::object::data::ObjectData>
        {
            #block
        }

        #[cfg(not(target_os = "wasi"))]
        #[tokio::main]
        pub async fn main() {
            use ipis::object::IntoObjectData;

            // infer test inputs
            let mut inputs = ().__into_object_data();

            match __ipwis_main_async(inputs).await {
                Ok(_outputs) => {},
                Err(errors) => ::ipis::log::error!("{}", errors),
            }
        }
    })
}
