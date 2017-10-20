/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

pub extern crate gfx_hal as hal;
#[cfg(target_os = "macos")]
pub extern crate gfx_backend_metal as backend;
#[cfg(windows)]
pub extern crate gfx_backend_dx12 as backend;
#[cfg(target_os =  "linux")]
pub extern crate gfx_backend_vulkan as backend;
//pub extern crate gfx_device_gl as backend;
