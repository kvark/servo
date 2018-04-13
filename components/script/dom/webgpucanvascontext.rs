/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//TODO: URL here

//use canvas_traits::webgpu as w;
use dom::bindings::reflector::reflect_dom_object;
use dom::bindings::root::{DomRoot, LayoutDom};
use dom::htmlcanvaselement::HTMLCanvasElement;
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
        _window: &Window,
        _canvas: &HTMLCanvasElement,
        _size: Size2D<i32>,
    ) -> Option<DomRoot<WebGPUCanvasContext>> {
        /*let swap_chain = WebGPUSwapChain::new();

        let webgpu_chan =
        let webgl_chan = match window.webgl_chan() {
            Some(chan) => chan,
            None => return Err("WebGL initialization failed early on".into()),
        };

        let (sender, receiver) = webgl_channel().unwrap();
        webgl_chan.send(WebGLMsg::CreateContext(webgl_version, size, attrs, sender))
                  .unwrap();
        let result = receiver.recv().unwrap();

        result.map(|ctx_data| {
            WebGLRenderingContext {
                reflector_: Reflector::new(),
                webgl_sender: ctx_data.sender,
                webrender_image: Cell::new(None),
                share_mode: ctx_data.share_mode,
                webgl_version,
                glsl_version: ctx_data.glsl_version,
                limits: ctx_data.limits,
                canvas: Dom::from_ref(canvas),
                last_error: Cell::new(None),
                texture_unpacking_settings: Cell::new(TextureUnpacking::CONVERT_COLORSPACE),
                texture_unpacking_alignment: Cell::new(4),
                bound_framebuffer: MutNullableDom::new(None),
                bound_textures: DomRefCell::new(Default::default()),
                bound_texture_unit: Cell::new(constants::TEXTURE0),
                bound_buffer_array: MutNullableDom::new(None),
                bound_buffer_element_array: MutNullableDom::new(None),
                bound_attrib_buffers: DomRefCell::new(Default::default()),
                bound_renderbuffer: MutNullableDom::new(None),
                current_program: MutNullableDom::new(None),
                current_vertex_attrib_0: Cell::new((0f32, 0f32, 0f32, 1f32)),
                current_scissor: Cell::new((0, 0, size.width, size.height)),
                current_clear_color: Cell::new((0.0, 0.0, 0.0, 0.0)),
                extension_manager: WebGLExtensions::new(webgl_version)
            }
        })

        match receiver.recv().unwrap() {
            Ok(ctx_data) => {
                Some(reflect_dom_object(Box::new(ctx), window, binding::Wrap))
            }
            Err(msg) => {
                error!("Couldn't create WebGPUCanvasContext: {}", msg);
                None
            }
        }*/
        None //TODO
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
