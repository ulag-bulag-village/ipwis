pub mod memory {
    use ipis::core::anyhow::{anyhow, Result};
    use ipwis_modules_task_common_wasi::extern_data::ExternDataRef;
    use wasmtime::{AsContextMut, Caller, Instance, TypedFunc};

    pub use ipwis_modules_task_common_wasi::extrinsics::memory::*;

    pub type IpwisAlloc = TypedFunc<(ExternDataRef, ExternDataRef), ExternDataRef>;
    pub type IpwisAllocZeroed = TypedFunc<(ExternDataRef, ExternDataRef), ExternDataRef>;
    pub type IpwisDealloc = TypedFunc<(ExternDataRef, ExternDataRef, ExternDataRef), ()>;
    pub type IpwisRealloc =
        TypedFunc<(ExternDataRef, ExternDataRef, ExternDataRef, ExternDataRef), ExternDataRef>;

    pub mod caller {
        use super::{super::common::caller as common, *};

        pub fn __builtin_memory<T>(caller: &mut Caller<T>) -> Result<::wasmtime::Memory> {
            caller
                .get_export(MEMORY)
                .and_then(::wasmtime::Extern::into_memory)
                .ok_or_else(|| anyhow!("failed to find `{MEMORY}` export"))
        }

        pub fn __alloc<T>(caller: &mut Caller<T>) -> Result<IpwisAlloc> {
            common::load_extern(caller, IPWIS_ALLOC)
        }

        pub fn __alloc_zeroed<T>(caller: &mut Caller<T>) -> Result<IpwisAllocZeroed> {
            common::load_extern(caller, IPWIS_ALLOC_ZEROED)
        }

        pub fn __dealloc<T>(caller: &mut Caller<T>) -> Result<IpwisDealloc> {
            common::load_extern(caller, IPWIS_DEALLOC)
        }

        pub fn __realloc<T>(caller: &mut Caller<T>) -> Result<IpwisRealloc> {
            common::load_extern(caller, IPWIS_REALLOC)
        }
    }

    pub mod instance {
        use super::{super::common::instance as common, *};

        pub fn __builtin_memory<S>(instance: &Instance, mut store: S) -> Result<::wasmtime::Memory>
        where
            S: AsContextMut,
        {
            instance
                .get_export(&mut store, "memory")
                .and_then(::wasmtime::Extern::into_memory)
                .ok_or_else(|| anyhow!("failed to find `{MEMORY}` export"))
        }

        pub fn __alloc<S>(instance: &Instance, store: S) -> Result<IpwisAlloc>
        where
            S: AsContextMut,
        {
            common::load_extern(instance, store, IPWIS_ALLOC)
        }

        pub fn __alloc_zeroed<S>(instance: &Instance, store: S) -> Result<IpwisAllocZeroed>
        where
            S: AsContextMut,
        {
            common::load_extern(instance, store, IPWIS_ALLOC_ZEROED)
        }

        pub fn __dealloc<S>(instance: &Instance, store: S) -> Result<IpwisDealloc>
        where
            S: AsContextMut,
        {
            common::load_extern(instance, store, IPWIS_DEALLOC)
        }

        pub fn __realloc<S>(instance: &Instance, store: S) -> Result<IpwisRealloc>
        where
            S: AsContextMut,
        {
            common::load_extern(instance, store, IPWIS_REALLOC)
        }
    }
}

pub mod syscall {
    use ipis::core::anyhow::Result;
    use ipwis_modules_task_common_wasi::extern_data::ExternDataRef;
    use wasmtime::{AsContextMut, Caller, Instance, TypedFunc};

    pub use ipwis_modules_task_common_wasi::extrinsics::syscall::*;

    pub type IpwisSyscall = TypedFunc<IpwisSyscallArgs, ExternDataRef>;
    pub type IpwisSyscallArgs = (
        /* handler */ ExternDataRef,
        /* inputs */ ExternDataRef,
        /* outputs */ ExternDataRef,
        /* errors */ ExternDataRef,
    );

    pub mod caller {
        use super::{super::common::caller as common, *};

        pub fn __syscall<T>(caller: &mut Caller<T>) -> Result<IpwisSyscall> {
            common::load_extern(caller, SYSCALL)
        }
    }

    pub mod instance {
        use super::{super::common::instance as common, *};

        pub fn __syscall<S>(instance: &Instance, store: S) -> Result<IpwisSyscall>
        where
            S: AsContextMut,
        {
            common::load_extern(instance, store, SYSCALL)
        }
    }

    mod impls {
        use ipis::log::warn;
        use ipwis_modules_task_common_wasi::interrupt_id::InterruptId;
        use rkyv::AlignedVec;

        use crate::{
            memory::{IpwisMemory, Memory},
            task_ctx::IpwisTaskCtx,
        };

        use super::*;

        pub async fn __syscall(
            mut caller: Caller<'_, IpwisTaskCtx>,
            handler: ExternDataRef,
            inputs: ExternDataRef,
            outputs: ExternDataRef,
            errors: ExternDataRef,
        ) -> ExternDataRef {
            let mut memory = unsafe {
                // allow interior mutability
                match IpwisMemory::with_caller(::core::mem::transmute::<
                    _,
                    &mut Caller<'static, IpwisTaskCtx>,
                >(&mut caller))
                {
                    Ok(memory) => memory,
                    Err(error) => {
                        warn!("{}", error);
                        return SYSCALL_ERR_FATAL;
                    }
                }
            };

            async unsafe fn try_syscall<'a>(
                caller: &mut Caller<'a, IpwisTaskCtx>,
                memory: &mut IpwisMemory,
                handler: ExternDataRef,
                inputs: ExternDataRef,
            ) -> Result<AlignedVec> {
                let handler = {
                    let data = ::core::mem::transmute(memory.load_doubled(handler)?); // ignore `memory` lifetime
                    InterruptId(::core::str::from_utf8(data)?)
                };
                let inputs: &[u8] = {
                    ::core::mem::transmute(memory.load_doubled(inputs)?) // ignore `memory` lifetime
                };

                caller
                    .data_mut()
                    .interrupt_handler_state
                    .syscall_raw(memory, handler, inputs)
                    .await
            }

            unsafe {
                match try_syscall(&mut caller, &mut memory, handler, inputs).await {
                    Ok(buf) => match memory.dump_to(&buf, outputs).await {
                        Ok(()) => SYSCALL_OK,
                        Err(error) => {
                            warn!("{}", error);
                            SYSCALL_ERR_FATAL
                        }
                    },
                    Err(error) => match memory.dump_error_to(error, errors).await {
                        Ok(()) => SYSCALL_ERR_NORMAL,
                        Err(error) => {
                            warn!("{}", error);
                            SYSCALL_ERR_FATAL
                        }
                    },
                }
            }
        }
    }

    pub mod linker {
        use wasmtime::Linker;

        use crate::task_ctx::IpwisTaskCtx;

        use super::*;

        pub fn __syscall(linker: &mut Linker<IpwisTaskCtx>) -> Result<()> {
            linker
                .func_wrap4_async(
                    MODULE,
                    SYSCALL,
                    |caller, handler, inputs, outputs, errors| {
                        Box::new(super::impls::__syscall(
                            caller, handler, inputs, outputs, errors,
                        ))
                    },
                )
                .map(|_| ())
        }
    }
}

mod common {
    pub mod caller {
        pub fn load_extern<T, Params, Results>(
            caller: &mut ::wasmtime::Caller<T>,
            name: &'static str,
        ) -> ::ipis::core::anyhow::Result<::wasmtime::TypedFunc<Params, Results>>
        where
            Params: ::wasmtime::WasmParams,
            Results: ::wasmtime::WasmResults,
        {
            caller
                .get_export(name)
                .and_then(::wasmtime::Extern::into_func)
                .ok_or_else(|| ::ipis::core::anyhow::anyhow!("failed to find `{}` export", name))
                .and_then(|e| e.typed(&caller).map_err(Into::into))
        }
    }

    pub mod instance {
        pub fn load_extern<S, Params, Results>(
            instance: &::wasmtime::Instance,
            mut store: S,
            name: &'static str,
        ) -> ::ipis::core::anyhow::Result<::wasmtime::TypedFunc<Params, Results>>
        where
            S: ::wasmtime::AsContextMut,
            Params: ::wasmtime::WasmParams,
            Results: ::wasmtime::WasmResults,
        {
            instance
                .get_export(&mut store, name)
                .and_then(::wasmtime::Extern::into_func)
                .ok_or_else(|| ::ipis::core::anyhow::anyhow!("failed to find `{}` export", name))
                .and_then(|e| e.typed(&store).map_err(Into::into))
        }
    }
}
