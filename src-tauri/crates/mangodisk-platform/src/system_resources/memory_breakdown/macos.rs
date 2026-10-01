//! macOS counters from the Mach virtual-memory statistics and `proc_pid_rusage`.
use std::{ffi::CString, mem::size_of};

use super::{
    MemoryBreakdownSnapshot, MemoryCategories, MemoryPressureLevel, ProcessFootprint,
    ProcessFootprintSnapshot, ProcessMemoryMetric,
};

/// Replaces the generic overview with Activity Monitor's categories when the
/// kernel statistics are readable; otherwise the generic values stay.
pub(super) fn refine(mut snapshot: MemoryBreakdownSnapshot) -> MemoryBreakdownSnapshot {
    snapshot.pressure = pressure_level();
    let (Some(page_size), Some(statistics)) = (page_size(), vm_statistics()) else {
        return snapshot;
    };
    let pages = |count: u32| u64::from(count).saturating_mul(page_size);
    // Application memory is anonymous pages that are not purgeable; purgeable and
    // file-backed pages are cache the system drops on demand.
    let categories = MemoryCategories {
        application_bytes: pages(
            statistics
                .internal_page_count
                .saturating_sub(statistics.purgeable_count),
        ),
        wired_bytes: pages(statistics.wire_count),
        compressed_bytes: pages(statistics.compressor_page_count),
        cached_bytes: pages(statistics.external_page_count)
            .saturating_add(pages(statistics.purgeable_count)),
    };
    let used = categories
        .application_bytes
        .saturating_add(categories.wired_bytes)
        .saturating_add(categories.compressed_bytes);
    snapshot.used_bytes = used.min(snapshot.total_bytes);
    snapshot.free_bytes = pages(
        statistics
            .free_count
            .saturating_sub(statistics.speculative_count),
    )
    .min(snapshot.total_bytes);
    snapshot.categories = Some(categories);
    snapshot
}

fn page_size() -> Option<u64> {
    let size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    u64::try_from(size).ok().filter(|size| *size > 0)
}

fn vm_statistics() -> Option<libc::vm_statistics64> {
    // libc deprecates these in favor of the mach2 crate; the declarations match
    // the one in cpu.rs and avoid a dependency for two stable libSystem calls.
    unsafe extern "C" {
        fn mach_host_self() -> libc::mach_port_t;
        fn mach_port_deallocate(
            task: libc::mach_port_t,
            name: libc::mach_port_t,
        ) -> libc::kern_return_t;
        static mach_task_self_: libc::mach_port_t;
    }
    let mut statistics: libc::vm_statistics64 = unsafe { std::mem::zeroed() };
    let mut count = libc::HOST_VM_INFO64_COUNT;
    let status = unsafe {
        // Every call to mach_host_self adds a send right that must be released.
        let port = mach_host_self();
        let status = libc::host_statistics64(
            port,
            libc::HOST_VM_INFO64,
            (&mut statistics as *mut libc::vm_statistics64).cast(),
            &mut count,
        );
        mach_port_deallocate(mach_task_self_, port);
        status
    };
    (status == libc::KERN_SUCCESS && count == libc::HOST_VM_INFO64_COUNT).then_some(statistics)
}

fn pressure_level() -> Option<MemoryPressureLevel> {
    let name = CString::new("kern.memorystatus_vm_pressure_level").ok()?;
    let mut level: libc::c_int = 0;
    let mut length = size_of::<libc::c_int>();
    let status = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            (&mut level as *mut libc::c_int).cast(),
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    };
    if status != 0 {
        return None;
    }
    // The kernel reports 1 (normal), 2 (warning), and 4 (critical).
    Some(match level {
        1 => MemoryPressureLevel::Normal,
        2 => MemoryPressureLevel::Warning,
        _ => MemoryPressureLevel::Critical,
    })
}

/// Physical footprint is the figure Activity Monitor shows. The kernel refuses it
/// for processes of other users, which then keep their resident size.
pub(super) fn footprints(mut processes: Vec<ProcessFootprint>) -> ProcessFootprintSnapshot {
    for process in &mut processes {
        match physical_footprint(process.pid) {
            Some(bytes) => process.bytes = bytes,
            None => process.approximate = true,
        }
    }
    ProcessFootprintSnapshot {
        metric: ProcessMemoryMetric::Footprint,
        processes,
    }
}

fn physical_footprint(pid: u32) -> Option<u64> {
    let pid = libc::c_int::try_from(pid).ok()?;
    let mut usage: libc::rusage_info_v2 = unsafe { std::mem::zeroed() };
    let status = unsafe {
        libc::proc_pid_rusage(
            pid,
            libc::RUSAGE_INFO_V2,
            (&mut usage as *mut libc::rusage_info_v2).cast(),
        )
    };
    (status == 0).then_some(usage.ri_phys_footprint)
}
