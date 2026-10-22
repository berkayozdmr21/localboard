//! An owned, non-inheritable Windows job. Closing its final handle kills members.
use crate::Result;
use std::{
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    process::Child,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};

pub struct Job(OwnedHandle);
impl Job {
    pub fn new() -> Result<Self> {
        // SAFETY: null security/name pointers create an unnamed, non-inheritable job.
        let raw = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if raw.is_null() {
            return Err(format!(
                "Windows süreç grubu oluşturulamadı: {}",
                std::io::Error::last_os_error()
            ));
        }
        // SAFETY: this uniquely owned valid handle is released by OwnedHandle.
        let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
        // SAFETY: this Win32 POD structure is valid when zero-initialized.
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: pointer and byte size match the specified information class.
        if unsafe {
            SetInformationJobObject(
                raw,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of_val(&info) as u32,
            )
        } == 0
        {
            return Err(format!(
                "Windows süreç grubu ayarlanamadı: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(Self(handle))
    }
    pub fn assign(&self, child: &Child) -> Result<()> {
        // SAFETY: both handles remain owned and live for the duration of the call.
        if unsafe { AssignProcessToJobObject(self.0.as_raw_handle(), child.as_raw_handle()) } == 0 {
            return Err(format!(
                "Süreç gruba eklenemedi: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }
    pub fn stop(&self) -> Result<()> {
        // SAFETY: the live handle belongs exclusively to this service's job.
        if unsafe { TerminateJobObject(self.0.as_raw_handle(), 1) } == 0 {
            return Err(format!(
                "Süreç grubu durdurulamadı: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }
}
