/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//TODO: URL here

use canvas_traits::webgpu as w;
use dom::bindings::codegen::Bindings::WebGPUBinding as binding;
use dom::bindings::reflector::{DomObject, Reflector, reflect_dom_object};
use dom::bindings::root::DomRoot;
use dom::window::Window;
use dom::webgpudevice::WebGPUDevice;
use dom_struct::dom_struct;



#[dom_struct]
pub struct WebGPU {
    reflector_: Reflector,
    info: w::InstanceInfo,
    #[ignore_malloc_size_of = "Defined in ipc-channel"]
    sender: w::WebGPUMainChan,
}

impl WebGPU {
    #[allow(unrooted_must_root)]
    pub fn Instance(window: &Window) -> DomRoot<Self> {
        let webgpu_chan = match window.webgpu_chan() {
            Some(chan) => chan,
            None => {
                panic!("WebGPU initialization failed early on");
            }
        };

        let (sender, receiver) = w::webgpu_channel().unwrap();
        let msg = w::Message::Init {
            result: sender,
        };
        webgpu_chan.send(msg).unwrap();
        let data = match receiver.recv().unwrap() {
            Ok(data) => data,
            Err(e) => {
                panic!("WebGPU server error, no response for Init: {:?}", e);
            }
        };

        let object = WebGPU {
            reflector_: Reflector::new(),
            info: data,
            sender: webgpu_chan,
        };
        reflect_dom_object(Box::new(object), window, binding::Wrap)
    }

    pub fn CreateDevice(
        &self, _desc: &binding::WebGPUDeviceDescriptor
    ) -> DomRoot<WebGPUDevice> {
        let (sender, receiver) = w::webgpu_channel().unwrap();
        let msg = w::Message::CreateDevice {
            //TODO: descriptor
            result: sender,
        };
        self.sender.send(msg).unwrap();

        match receiver.recv().unwrap() {
            Ok(data) => {
                WebGPUDevice::new(
                    self.global().as_window(),
                    data,
                    self.info.clone(),
                    self.sender.clone(),
                )
            }
            Err(e) => {
                panic!("WebGPU server error, no response for CreateDevice: {:?}", e);
            }
        }
    }
}
