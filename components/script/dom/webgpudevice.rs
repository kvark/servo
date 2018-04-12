/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//TODO: URL here

use canvas_traits::webgpu as w;
use dom::bindings::reflector::Reflector;
use dom_struct::dom_struct;


#[dom_struct]
pub struct WebGPUDevice {
    reflector_: Reflector,
    id: w::DeviceId,
    #[ignore_malloc_size_of = "Defined in ipc-channel"]
    sender: w::WebGPUMainChan,
}

impl WebGPUDevice {
    pub fn id(&self) -> w::DeviceId {
        self.id
    }
}

impl Drop for WebGPUDevice {
    fn drop(&mut self) {
        //TODO
    }
}
