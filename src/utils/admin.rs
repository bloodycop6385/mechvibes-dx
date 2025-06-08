// Admin privilege detection utilities for Windows
#[cfg(target_os = "windows")]
use winapi::um::processthreadsapi::GetCurrentProcess;
#[cfg(target_os = "windows")]
use winapi::um::securitybaseapi::GetTokenInformation;
#[cfg(target_os = "windows")]
use winapi::um::winnt::{ TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY };
#[cfg(target_os = "windows")]
use winapi::um::handleapi::CloseHandle;
#[cfg(target_os = "windows")]
use winapi::um::processthreadsapi::OpenProcessToken;

/// Check if the current process is running with administrator privileges
/// Returns true if elevated (administrator), false otherwise
/// On non-Windows platforms, always returns false
pub fn is_running_as_admin() -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::mem;
        use std::ptr;

        unsafe {
            let mut token_handle = ptr::null_mut();

            // Get the access token for the current process
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle) == 0 {
                return false;
            }

            let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
            let mut size = 0;

            // Get the elevation information from the token
            let result = GetTokenInformation(
                token_handle,
                TokenElevation,
                &mut elevation as *mut _ as *mut _,
                mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut size
            );

            // Clean up the token handle
            CloseHandle(token_handle);

            if result == 0 {
                return false;
            }

            elevation.TokenIsElevated != 0
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// Get a human-readable description of the current privilege level
pub fn get_privilege_level_description() -> &'static str {
    if is_running_as_admin() { "Administrator (Elevated)" } else { "Standard User" }
}

/// Check if administrator privileges are recommended
/// This helps determine if the user should be warned about potential limitations
pub fn should_recommend_admin_privileges() -> bool {
    #[cfg(target_os = "windows")]
    {
        // On Windows, recommend admin privileges if not currently running as admin
        // Since the app is now built with admin privileges by default,
        // this mainly helps detect if UAC was declined
        !is_running_as_admin()
    }

    #[cfg(not(target_os = "windows"))]
    {
        false // Not relevant on non-Windows platforms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privilege_detection() {
        // Test that the function doesn't panic
        let is_admin = is_running_as_admin();
        let description = get_privilege_level_description();

        println!("Running as admin: {}", is_admin);
        println!("Privilege level: {}", description);

        // Basic sanity checks
        assert!(description == "Administrator (Elevated)" || description == "Standard User");

        if is_admin {
            assert_eq!(description, "Administrator (Elevated)");
        } else {
            assert_eq!(description, "Standard User");
        }
    }
}
