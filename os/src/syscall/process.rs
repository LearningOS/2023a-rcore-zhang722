//! Process management syscalls
use core::usize;

use crate::{
    config::MAX_SYSCALL_NUM,
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus, current_user_token,
        get_current_task_info, TASK_MANAGER,
    },
    mm::{translated_byte_buffer, byte_buffer_assign, VirtAddr, VPNRange, MapPermission},
    timer::{get_time_us, get_time_ms},
    
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let tv = TimeVal {
        sec: us / 1000000,
        usec: us % 1000000,
    };
    let tv_buffer: &[u8] = unsafe {
        core::slice::from_raw_parts(
            &tv as *const _ as *const u8, 
            16,
        )
    };
    let token = current_user_token();
    let mut buffer = translated_byte_buffer(token, _ts as *const u8, 16);
    byte_buffer_assign(tv_buffer, &mut buffer);
 
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    let info = get_current_task_info();
    let mut syscall_times = [0u32; MAX_SYSCALL_NUM];
    for (&syscall_id, syscall_time) in info.sys_call_ids.iter().zip(info.sys_call_nums) {
        syscall_times[syscall_id] = syscall_time as u32;
    }
    let current_time = get_time_ms();
    let ti = TaskInfo {
        status: TaskStatus::Running,
        time: current_time - info.time.unwrap_or(0),
        syscall_times,
    };
    
    let ti_buffer: &[u8] = unsafe {
        core::slice::from_raw_parts(
            &ti as *const _ as *const u8, 
            core::mem::size_of::<TaskInfo>(),
        )
    };
    let token = current_user_token();
    let mut buffer = translated_byte_buffer(
        token, 
        _ti as *const u8, 
        core::mem::size_of::<TaskInfo>());
    byte_buffer_assign(ti_buffer, &mut buffer);

    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    if !VirtAddr::from(_start).aligned() {
        error!("start address not aligned");
        return -1;
    }
    if _port & !0x7 != 0 {
        error!("port & !0x7 != 0");
        return -1;
    }
    if _port & 0x7 == 0 {
        error!("port & 0x7 = 0");
        return -1;
    }
    // check the range [start, start + len)
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    let svpn = VirtAddr::from(_start).floor();
    let evpn = VirtAddr::from(_start + _len).ceil();
    let vpns = VPNRange::new(svpn, evpn);
    for vpn in vpns {
       if let Some(pte) = inner.tasks[current].memory_set.translate(vpn) {
            if pte.is_valid() {
                return -1;
            }
       }
    }
    inner.tasks[current].memory_set.insert_framed_area(
        svpn.into(),
        evpn.into(),
        MapPermission::from_bits_truncate((_port << 1) as u8) | MapPermission::U
    );  
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    if !VirtAddr::from(_start).aligned() {
        error!("start address not aligned");
        return -1;
    }
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    let svpn = VirtAddr::from(_start).floor();
    let evpn = VirtAddr::from(_start + _len).ceil();
    let vpns = VPNRange::new(svpn, evpn);
    for vpn in vpns {
        if let Some(pte) = inner.tasks[current].memory_set.translate(vpn) {
            if !pte.is_valid() {
                return -1;
            }
            inner.tasks[current].memory_set.page_table().unmap(vpn);
        } else {
            return -1;
        }
    }
    0
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
