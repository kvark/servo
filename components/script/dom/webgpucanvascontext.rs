/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//TODO: URL here

use canvas_traits::webgpu as w;
use dom::bindings::codegen::Bindings::WebGPUCanvasContextBinding as binding;
use dom::bindings::reflector::reflect_dom_object;
use dom::bindings::root::{DomRoot, LayoutDom};
use dom::htmlcanvaselement::HTMLCanvasElement;
use dom::webgpudevice::WebGPUDevice;
use dom::window::Window;
use dom_struct::dom_struct;
use dom::webgpuswapchain::WebGPUSwapChain;
use script_layout_interface::HTMLCanvasDataSource;

use euclid::Size2D;
use webrender_api;


#[dom_struct]
pub struct WebGPUCanvasContext {
    swap_chain: WebGPUSwapChain,
    #[ignore_malloc_size_of = "Defined in webrender"]
    wr_image: webrender_api::ImageKey,
}

impl WebGPUCanvasContext {
    #[allow(unrooted_must_root)]
    pub fn new(
        window: &Window,
        _canvas: &HTMLCanvasElement,
        size: Size2D<i32>,
        device: &WebGPUDevice,
    ) -> Option<DomRoot<WebGPUCanvasContext>> {
        let webgpu_chan = match window.webgpu_chan() {
            Some(chan) => chan,
            None => {
                error!("WebGPU initialization failed early on");
                return None
            }
        };

        let (sender, receiver) = w::webgpu_channel().unwrap();
        let msg = w::Message::CreateSwapChain {
            device: device.id(),
            size: size.to_u32(),
            result: sender,
        };
        webgpu_chan.send(msg).unwrap();
        let data = match receiver.recv().unwrap() {
            Ok(data) => data,
            Err(e) => {
                error!("WebGPU server error, no response for CreateSwapChain: {:?}", e);
                return None
            }
        };

        let object = WebGPUCanvasContext {
            swap_chain: WebGPUSwapChain::new_internal(
                device.id(),
                data.id,
                webgpu_chan.clone(),
            ),
            wr_image: data.image_key,
        };
        Some(reflect_dom_object(Box::new(object), window, binding::Wrap))
    }

    pub fn recreate(&self, _size: Size2D<i32>) {
        //TODO
    }

    fn layout_handle(&self) -> webrender_api::ImageKey {
        self.wr_image
    }
}

impl Drop for WebGPUCanvasContext {
    fn drop(&mut self) {
        //TODO
    }
}


pub trait LayoutCanvasWebGPUCanvasContextHelpers {
    #[allow(unsafe_code)]
    unsafe fn canvas_data_source(&self) -> HTMLCanvasDataSource;
}

impl LayoutCanvasWebGPUCanvasContextHelpers for LayoutDom<WebGPUCanvasContext> {
    #[allow(unsafe_code)]
    unsafe fn canvas_data_source(&self) -> HTMLCanvasDataSource {
        HTMLCanvasDataSource::WebGPU((*self.unsafe_get()).layout_handle())
    }
}
