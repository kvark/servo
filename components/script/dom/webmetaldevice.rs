/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::WebMetalDeviceRequest;
use dom::bindings::codegen::Bindings::WebMetalDeviceBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflectable, Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::webmetalbuffer::WebMetalBuffer;
use dom::webmetalrenderpipelinestate::WebMetalRenderPipelineState;
use ipc_channel::ipc::{self, IpcSender};
use js::jsapi::{JSContext, JSObject};
use webmetal;

#[dom_struct]
pub struct WebMetalDevice {
    reflector: Reflector,
    #[ignore_heap_size_of = "Defined in ipc-channel"]
    ipc_device: IpcSender<WebMetalDeviceRequest>,
}

impl WebMetalDevice {
    pub fn new(global: &GlobalScope,
               ipc_device: IpcSender<WebMetalDeviceRequest>)
               -> Root<WebMetalDevice> {
        let object = box WebMetalDevice {
            reflector: Reflector::new(),
            ipc_device: ipc_device,
        };
        reflect_dom_object(object, global, binding::Wrap)
    }
}

impl binding::WebMetalDeviceMethods for WebMetalDevice {
    #[allow(unsafe_code)]
    unsafe fn MakeBuffer(&self, _cx: *mut JSContext, _size: u32, _data: *mut JSObject)
                         -> Root<WebMetalBuffer> {
        WebMetalBuffer::new(&self.global())
    }

    fn MakeRenderPipelineState(&self, desc: &binding::RenderPipelineDesc)
                               -> Root<WebMetalRenderPipelineState> {
        let (sender_vs, receiver_vs) = ipc::channel().unwrap();
        let (sender_fs, receiver_fs) = ipc::channel().unwrap();
        let req_vs = WebMetalDeviceRequest::MakeShader(sender_vs,
                                                       (*desc.vertexFunction).to_string(),
                                                       webmetal::ShaderType::Vertex);
        let req_fs = WebMetalDeviceRequest::MakeShader(sender_fs,
                                                       (*desc.fragmentFunction).to_string(),
                                                       webmetal::ShaderType::Fragment);
        self.ipc_device.send(req_vs).unwrap();
        self.ipc_device.send(req_fs).unwrap();
        let desc = webmetal::PipelineDesc {
            fun_vertex: receiver_vs.recv().unwrap().unwrap(),
            fun_fragment: receiver_fs.recv().unwrap().unwrap(),
        };
        WebMetalRenderPipelineState::new(&self.global(), desc, self.ipc_device.clone())
    }
}
