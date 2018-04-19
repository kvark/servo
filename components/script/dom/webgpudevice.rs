/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//TODO: URL here

use canvas_traits::webgpu as w;
use dom::bindings::codegen::Bindings::WebGPUDeviceBinding as binding;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::bindings::root::DomRoot;
use dom_struct::dom_struct;
use dom::window::Window;


#[dom_struct]
pub struct WebGPUDevice {
    reflector_: Reflector,
    id: (w::DeviceId, w::QueueId),
    info: w::InstanceInfo,
    #[ignore_malloc_size_of = "Defined in ipc-channel"]
    sender: w::WebGPUMainChan,
}

impl WebGPUDevice {
    #[allow(unrooted_must_root)]
    pub fn new(
        window: &Window,
        dev_info: w::DeviceInfo, instance_info: w::InstanceInfo,
        sender: w::WebGPUMainChan,
    ) -> DomRoot<Self> {
        let object = WebGPUDevice {
            reflector_: Reflector::new(),
            id: (dev_info.id, dev_info.queue_id),
            info: instance_info,
            sender,
        };
        reflect_dom_object(Box::new(object), window, binding::Wrap)
    }

    pub fn id(&self) -> w::DeviceId {
        self.id.0
    }

    pub fn queue_id(&self) -> w::QueueId {
        self.id.1
    }
}

impl WebGPUDevice {
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
}

impl Drop for WebGPUDevice {
    fn drop(&mut self) {
        //TODO
    }
}
