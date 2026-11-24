//! The pinned Linux CLI uses jemalloc. Its monotone per-worker allocation clock
//! includes provider allocations whose native types are opaque to the session.
#[cfg(all(
    target_os = "linux",
    any(
        target_arch = "aarch64",
        target_arch = "x86_64",
        target_arch = "powerpc64",
        target_arch = "riscv64"
    )
))]
pub(super) fn bytes() -> Option<u64> {
    use std::cell::OnceCell;
    use std::ptr;
    std::thread_local! { static COUNTER: OnceCell<Option<*const u64>> = const { OnceCell::new() }; }
    COUNTER.with(|counter| {
        let address = (*counter.get_or_init(|| {
            let mut p: *const u64 = ptr::null();
            let mut size = size_of_val(&p);
            // SAFETY: mallctl writes one pointer into p. jemalloc owns that counter
            // for the life of the current thread; it never crosses the thread boundary.
            let result = unsafe {
                tikv_jemalloc_sys::mallctl(
                    c"thread.allocatedp".as_ptr(),
                    (&raw mut p).cast(),
                    &mut size,
                    ptr::null_mut(),
                    0,
                )
            };
            (result == 0 && !p.is_null() && size == size_of_val(&p)).then_some(p)
        }))?;
        // SAFETY: this thread is the sole updater of the live jemalloc counter.
        Some(unsafe { ptr::read_volatile(address) })
    })
}

#[cfg(not(all(
    target_os = "linux",
    any(
        target_arch = "aarch64",
        target_arch = "x86_64",
        target_arch = "powerpc64",
        target_arch = "riscv64"
    )
)))]
pub(super) fn bytes() -> Option<u64> {
    None
}
