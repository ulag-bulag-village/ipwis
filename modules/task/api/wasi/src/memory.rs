use ipis::{
    async_trait::async_trait,
    bytecheck::CheckBytes,
    core::{
        anyhow::{bail, Result},
        signed::{IsSigned, Serializer},
    },
    rkyv::{
        de::deserializers::SharedDeserializeMap, validation::validators::DefaultValidator, Archive,
        Deserialize, Serialize,
    },
};
use ipwis_modules_task_common_wasi::{
    extern_data::{ExternData, ExternDataRef},
    interrupt_id::InterruptId,
};
use wasmtime::{AsContext, AsContextMut, Caller, Instance, Trap};

use crate::{
    interrupt_handler_state::IpwisInterruptHandler,
    intrinsics::memory::{self, IpwisAlloc, IpwisAllocZeroed, IpwisDealloc, IpwisRealloc},
    task_ctx::IpwisTaskCtx,
};

pub type IpwisMemory = IpwisMemoryInner<&'static mut Caller<'static, IpwisTaskCtx>>;

pub struct IpwisMemoryInner<S> {
    pub store: S,
    memory: ::wasmtime::Memory,
    alloc: IpwisAlloc,
    alloc_zeroed: IpwisAllocZeroed,
    dealloc: IpwisDealloc,
    realloc: IpwisRealloc,
}

impl<'a, 'c, T> IpwisMemoryInner<&'c mut Caller<'a, T>> {
    pub fn with_caller(caller: &'c mut Caller<'a, T>) -> Result<Self> {
        Ok(Self {
            memory: memory::caller::__builtin_memory(caller)?,
            alloc: memory::caller::__alloc(caller)?,
            alloc_zeroed: memory::caller::__alloc_zeroed(caller)?,
            dealloc: memory::caller::__dealloc(caller)?,
            realloc: memory::caller::__realloc(caller)?,
            store: caller,
        })
    }
}

impl<S> IpwisMemoryInner<S>
where
    S: AsContextMut,
{
    pub fn with_instance(instance: &Instance, mut store: S) -> Result<Self> {
        Ok(Self {
            memory: memory::instance::__builtin_memory(instance, &mut store)?,
            alloc: memory::instance::__alloc(instance, &mut store)?,
            alloc_zeroed: memory::instance::__alloc_zeroed(instance, &mut store)?,
            dealloc: memory::instance::__dealloc(instance, &mut store)?,
            realloc: memory::instance::__realloc(instance, &mut store)?,
            store,
        })
    }
}

#[async_trait]
impl<S> Memory for IpwisMemoryInner<S>
where
    S: AsContextMut + Send + Sync,
    S::Data: Send,
{
    fn host_check(&self, data: ExternData) -> Result<()> {
        if data.is_null() {
            bail!("data is null");
        }

        if ((data.ptr + data.len) as usize) >= self.size() {
            bail!("data overflow");
        }

        Ok(())
    }

    unsafe fn host_ptr_unchecked<T>(&self, ptr: ExternDataRef) -> *const T {
        self.memory.data_ptr(&self.store).add(ptr as usize) as *const T
    }

    unsafe fn host_ptr_mut_unchecked<T>(&mut self, ptr: ExternDataRef) -> *mut T {
        self.memory.data_ptr(&self.store).add(ptr as usize) as *mut T
    }

    async fn dump(&mut self, data: &[u8]) -> Result<ExternData> {
        let data_len = data.len();

        let len = data_len as ExternDataRef;
        let align = 1; // u8

        // safety: TODO: checking linear memory's limit
        let ptr = unsafe { self.alloc(len, align) }.await?;

        // safety: the source and destination are already checked
        unsafe { ::core::ptr::copy(data.as_ptr(), self.host_ptr_mut_unchecked(ptr), data_len) };
        Ok(ExternData { ptr, len })
    }
}

#[allow(dead_code)] // TODO: make more **safe** functions to get rid of it
impl<S, T> IpwisMemoryInner<S>
where
    S: AsContextMut<Data = T>,
    T: Send,
{
    async unsafe fn alloc(
        &mut self,
        size: ExternDataRef,
        align: ExternDataRef,
    ) -> Result<ExternDataRef, Trap>
    where
        T: Send,
    {
        self.alloc.call_async(&mut self.store, (size, align)).await
    }

    async unsafe fn alloc_zeroed(
        &mut self,
        size: ExternDataRef,
        align: ExternDataRef,
    ) -> Result<ExternDataRef, Trap>
    where
        T: Send,
    {
        self.alloc_zeroed
            .call_async(&mut self.store, (size, align))
            .await
    }

    async unsafe fn dealloc(
        &mut self,
        ptr: ExternDataRef,
        size: ExternDataRef,
        align: ExternDataRef,
    ) -> Result<(), Trap>
    where
        T: Send,
    {
        self.dealloc
            .call_async(&mut self.store, (ptr, size, align))
            .await
    }

    async unsafe fn realloc(
        &mut self,
        ptr: ExternDataRef,
        size: ExternDataRef,
        align: ExternDataRef,
        new_size: ExternDataRef,
    ) -> Result<ExternDataRef, Trap>
    where
        T: Send,
    {
        self.realloc
            .call_async(&mut self.store, (ptr, size, align, new_size))
            .await
    }
}

impl IpwisMemory {
    pub async fn get_interrupt_handler(
        &mut self,
        handler: InterruptId,
    ) -> Result<IpwisInterruptHandler> {
        self.store
            .data_mut()
            .interrupt_handler_state
            .get(handler)
            .await
    }
}

impl<S> IpwisMemoryInner<S>
where
    S: AsContext,
{
    fn size(&self) -> usize {
        self.memory.data_size(&self.store)
    }
}

#[async_trait]
pub trait Memory: Send + Sync {
    fn is_null(&self, data: ExternDataRef) -> Result<bool> {
        Ok((data as *const ExternData).is_null())
    }

    fn host_check(&self, data: ExternData) -> Result<()>;

    fn host_ptr<T>(&self, ptr: ExternDataRef) -> Result<*const T> {
        self.host_check(ExternData {
            ptr,
            // note: platform-specific types may break the code!
            len: ::core::mem::size_of::<T>().try_into()?,
        })
        // safety: checking is already done by `host_check`
        .map(|()| unsafe { self.host_ptr_unchecked(ptr) })
    }

    unsafe fn host_ptr_unchecked<T>(&self, ptr: ExternDataRef) -> *const T;

    fn host_ref<T>(&self, ptr: ExternDataRef) -> Result<&T> {
        self.host_ptr(ptr)
            // safety: checking is already done by `host_ptr`
            .map(|ptr| unsafe { &*ptr })
    }

    fn host_ptr_mut<T>(&mut self, ptr: ExternDataRef) -> Result<*mut T> {
        self.host_check(ExternData {
            ptr,
            // note: platform-specific types may break the code!
            len: ::core::mem::size_of::<T>().try_into()?,
        })
        // safety: checking is already done by `host_check`
        .map(|()| unsafe { self.host_ptr_mut_unchecked(ptr) })
    }

    unsafe fn host_ptr_mut_unchecked<T>(&mut self, ptr: ExternDataRef) -> *mut T;

    fn host_ref_mut<T>(&mut self, ptr: ExternDataRef) -> Result<&mut T> {
        self.host_ptr_mut(ptr)
            // safety: checking is already done by `host_ptr_mut`
            .map(|ptr| unsafe { &mut *ptr })
    }

    fn load(&self, data: ExternData) -> Result<&[u8]> {
        self.host_check(data)
            // safety: checking is already done by `host_check`
            .map(|()| unsafe {
                let ptr = self.host_ptr_unchecked(data.ptr);
                ::core::slice::from_raw_parts(ptr, data.len as usize)
            })
    }

    fn load_mut(&mut self, data: ExternData) -> Result<&mut [u8]> {
        self.host_check(data)
            // safety: checking is already done by `host_check`
            .map(|()| unsafe {
                let ptr = self.host_ptr_mut_unchecked(data.ptr);
                ::core::slice::from_raw_parts_mut(ptr, data.len as usize)
            })
    }

    fn load_doubled(&self, data: ExternDataRef) -> Result<&[u8]> {
        // safety: checking is already done by `host_ptr`
        let data = unsafe { *self.host_ptr(data)? };
        self.load(data)
    }

    fn load_doubled_mut(&mut self, data: ExternDataRef) -> Result<&mut [u8]> {
        // safety: checking is already done by `host_ptr_mut`
        let data = unsafe { *self.host_ptr_mut(data)? };
        self.load_mut(data)
    }

    async fn dump(&mut self, data: &[u8]) -> Result<ExternData>;

    async fn dump_doubled(&mut self, data: &[u8]) -> Result<ExternData> {
        let data = self.dump(data).await?.as_bytes();
        self.dump(&data).await
    }

    async fn dump_doubled_null(&mut self) -> Result<ExternData> {
        let data = ExternData::default().as_bytes();
        self.dump_doubled(&data).await
    }

    async fn dump_doubled_object<T>(&mut self, data: &T) -> Result<ExternData>
    where
        T: Archive + Serialize<Serializer> + IsSigned + Clone + Send + Sync,
        <T as Archive>::Archived:
            for<'a> CheckBytes<DefaultValidator<'a>> + Deserialize<T, SharedDeserializeMap>,
    {
        self.dump_doubled(&data.to_bytes()?).await
    }

    async fn dump_to(&mut self, src: &[u8], dst: ExternDataRef) -> Result<()> {
        // safety: mutability makes this block thread-safe
        let dst = unsafe { ::core::mem::transmute(self.host_ref_mut::<ExternData>(dst)?) };
        self.dump_to_raw(src, dst).await
    }

    async fn dump_to_raw(&mut self, src: &[u8], dst: &mut ExternData) -> Result<()> {
        *dst = self.dump(src).await?;
        Ok(())
    }

    async fn dump_error_to(
        &mut self,
        err: ::ipis::core::anyhow::Error,
        dst: ExternDataRef,
    ) -> Result<()> {
        self.dump_to(&err.to_string().into_bytes(), dst).await
    }

    fn set_len(&mut self, len: ExternDataRef, dst: ExternDataRef) -> Result<()> {
        // safety: mutability makes this block thread-safe
        let dst = unsafe { ::core::mem::transmute(self.host_ref_mut::<ExternData>(dst)?) };
        self.set_len_raw(len, dst);
        Ok(())
    }

    fn set_len_raw(&mut self, len: ExternDataRef, dst: &mut ExternData) {
        dst.len = len
    }
}
