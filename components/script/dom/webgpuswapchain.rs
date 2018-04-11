/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//TODO: URL here

use canvas_traits::webgpu as w;
use dom::bindings::reflector::Reflector;
use dom::bindings::root::LayoutDom;
use dom_struct::dom_struct;
use script_layout_interface::HTMLCanvasDataSource;

use euclid::Size2D;
use webrender_api;
/*
use dom::bindings::codegen::Bindings::WebGLShaderBinding;
use dom::bindings::reflector::reflect_dom_object;
use dom::bindings::root::DomRoot;
use dom::bindings::str::DOMString;
use dom::window::Window;
*/


#[dom_struct]
pub struct WebGPUSwapchain {
    reflector_: Reflector,
    id: w::SwapchainId,
    #[ignore_malloc_size_of = "Defined in webrender"]
    wr_image: webrender_api::ImageKey,
    #[ignore_malloc_size_of = "Defined in ipc-channel"]
    sender: w::WebGPUMainChan,
}

impl WebGPUSwapchain {
    pub fn id(&self) -> w::SwapchainId {
        self.id
    }

    pub fn recreate(&self, _size: Size2D<i32>) {
        //TODO
    }

    fn layout_handle(&self) -> webrender_api::ImageKey {
        self.wr_image
    }
}

impl Drop for WebGPUSwapchain {
    fn drop(&mut self) {
        //TODO
    }
}


pub trait LayoutCanvasWebGPUSwapchainHelpers {
    #[allow(unsafe_code)]
    unsafe fn canvas_data_source(&self) -> HTMLCanvasDataSource;
}

impl LayoutCanvasWebGPUSwapchainHelpers for LayoutDom<WebGPUSwapchain> {
    #[allow(unsafe_code)]
    unsafe fn canvas_data_source(&self) -> HTMLCanvasDataSource {
        HTMLCanvasDataSource::WebGPU((*self.unsafe_get()).layout_handle())
    }
}
