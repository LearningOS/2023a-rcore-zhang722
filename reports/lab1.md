实现了获得系统调用时刻距离任务第一次被调度时刻的时长，任务使用的系统调用及调用次数功能。不采用桶计数统计系统调用次数，转而采用两个短数组进行统计，节约了开销。
在config.rs中定义系统调用数量，以节约统计开销
~~~Rust
// Custom
/// the number of current system calls
pub const SYSCALL_NUM :usize = 5;
~~~

由于TaskInfo中对系统调用的统计是用桶计数序完成的，而在OS实现中，采用了另一种计数方式，所以需要在syscall/process.rs中对得到的任务信息进行抽取并赋值到TaskInfo中
~~~Rust
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    let info = get_current_task_info();
    let mut syscall_times = [0u32; MAX_SYSCALL_NUM];
    for (&syscall_id, syscall_time) in info.sys_call_ids.iter().zip(info.sys_call_nums) {
        syscall_times[syscall_id] = syscall_time as u32;
    }
    let current_time = get_time_ms();
    unsafe {
        (*_ti).status = TaskStatus::Running;
        (*_ti).time = current_time - info.time.unwrap_or(0);
        (*_ti).syscall_times = syscall_times;
    }
    0
}
~~~

第一次运行的时间由TaskManager维护。用Option<usize>储存第一次运行的时间
~~~Rust
/// Get info
fn current_task_info(&self) -> TaskInfoBlock {
    let inner = self.inner.exclusive_access();
    let current = inner.current_task;

    inner.task_infos[current]
}

/// Update task info
fn update_task_info(&self, syscall_id: usize) -> isize {
    let mut inner = self.inner.exclusive_access();
    let current = inner.current_task;
    
    let info = &mut inner.task_infos[current];
    if let Some(idx) = info.sys_call_ids.iter().position(|&x| x == syscall_id) {
        info.sys_call_nums[idx] += 1;
        return 0;
    } else if let Some(idx) = info.sys_call_ids.iter().position(|&x| x == 0) {
        info.sys_call_ids[idx] = syscall_id;
        info.sys_call_nums[idx] += 1;
        return 0;
    }
    -1
    
}
~~~

调用次数由syscall函数进行维护
~~~Rust
let _ = task::update_current_task_info(syscall_id);
~~~