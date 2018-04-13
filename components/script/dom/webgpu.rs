/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//TODO: URL here

use canvas_traits::webgpu as w;
use dom::bindings::codegen::Bindings::WebGPUBinding as binding;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::bindings::root::DomRoot;
use dom::window::Window;
use dom::webgpudevice::WebGPUDevice;
use dom_struct::dom_struct;



#[dom_struct]
pub struct WebGPU {
    reflector_: Reflector,
    _info: w::InstanceInfo,
    #[ignore_malloc_size_of = "Defined in ipc-channel"]
    sender: w::WebGPUMainChan,
}

impl WebGPU {
    #[allow(unrooted_must_root)]
    pub fn new(
        window: &Window, info: w::InstanceInfo, sender: w::WebGPUMainChan
    ) -> DomRoot<Self> {
        let object = WebGPU {
            reflector_: Reflector::new(),
            _info: info,
            sender,
        };
        reflect_dom_object(Box::new(object), window, binding::Wrap)
    }
}

impl WebGPU {
    /*
    pub fn Instance(_win: &DomRoot<Window>) -> DomRoot<Self> {
        unimplemented!()
    }*/

    pub fn GetExtensions(&self) -> binding::WebGPUExtensions {
        binding::WebGPUExtensions {
            anisotropicFiltering: false,
        }
    }

    pub fn GetFeatures(&self) -> binding::WebGPUFeatures {
        binding::WebGPUFeatures {
            logicOp: false,
        }
    }

    pub fn GetLimits(&self) -> binding::WebGPULimits {
        binding::WebGPULimits {
            maxBindGroups: 1<<20,
        }
    }

    pub fn CreateDevice(
        &self, desc: &binding::WebGPUDeviceDescriptor
    ) -> DomRoot<WebGPUDevice> {
        unimplemented!()
    }
}
