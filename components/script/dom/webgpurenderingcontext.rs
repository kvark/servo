/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::cell::Cell;
use canvas_traits::webgpu::{
    WebGpuChan, WebGpuMsg, webgpu_channel,
    Presenter,
};
use dom::bindings::codegen::Bindings::WebGpuRenderingContextBinding as binding;
use dom::bindings::codegen::Bindings::WebGpuDeviceBinding::WebGpuFormat;
use dom::bindings::js::{JS, LayoutJS, Root};
use dom::bindings::reflector::{DomObject, Reflector, reflect_dom_object};
use dom::htmlcanvaselement::HTMLCanvasElement;
use dom::bindings::inheritance::Castable;
use dom::webgpuadapter::WebGpuAdapter;
use dom::webgpucommandqueue::WebGpuCommandQueue;
use dom::webgpuswapchain::WebGpuSwapchain;
use dom::window::Window;
use dom_struct::dom_struct;
use euclid::Size2D;
use script_layout_interface::HTMLCanvasDataSource;
use webrender_api;

static mut NEXT_EXTERNAL_IMAGE_ID: webrender_api::ExternalImageId = webrender_api::ExternalImageId(100);

#[dom_struct]
pub struct WebGpuRenderingContext {
    reflector_: Reflector,
    #[ignore_heap_size_of = "Channels are hard"]
    sender: WebGpuChan,
    canvas: JS<HTMLCanvasElement>,
    presenter: Presenter,
    adapters: Vec<Root<WebGpuAdapter>>,
    image_key: webrender_api::ImageKey,
    canvas_size: Cell<(Size2D<u32>, u32)>,
}

impl WebGpuRenderingContext {
    #[allow(unsafe_code)]
    fn new_internal(
        window: &Window,
        canvas: &HTMLCanvasElement,
    ) -> Result<WebGpuRenderingContext, String>
    {
        let (sender, receiver) = webgpu_channel().unwrap();
        let webgpu_chan = window.webgpu_chan();
        let size = canvas.get_size().cast().unwrap();

        let external_image_id = unsafe {
            let id = NEXT_EXTERNAL_IMAGE_ID;
            NEXT_EXTERNAL_IMAGE_ID.0 = id.0 + 1;
            id
        };

        webgpu_chan
            .send(WebGpuMsg::CreateContext {
                size,
                external_image_id,
                result: sender,
            })
            .unwrap();
        receiver
            .recv()
            .unwrap()
            .map(|init| {
                let sender = init.sender.clone();
                let adapters = init.adapters
                    .into_iter()
                    .map(|info| WebGpuAdapter::new(window, sender.clone(), info))
                    .collect();
                WebGpuRenderingContext {
                    reflector_: Reflector::new(),
                    sender,
                    canvas: JS::from_ref(canvas),
                    presenter: init.presenter,
                    adapters,
                    image_key: init.image_key,
                    canvas_size: Cell::new((size, 0)),
                }
            })
    }

    #[allow(unrooted_must_root)]
    pub fn new(
        window: &Window,
        canvas: &HTMLCanvasElement,
    ) -> Option<Root<WebGpuRenderingContext>> {
        match Self::new_internal(window, canvas) {
            Ok(ctx) => Some(reflect_dom_object(box ctx, window, binding::Wrap)),
            Err(msg) => {
                error!("Couldn't create WebGpuRenderingContext: {}", msg);
                //TODO: error event?
                None
            }
        }
    }

    pub fn recreate(&self) {
        //TODO
    }

    fn layout_handle(&self) -> webrender_api::ImageKey {
        let (size, stride) = self.canvas_size.get();
        let msg = WebGpuMsg::Present {
            image_key: self.image_key,
            external_image_id: self.presenter.id,
            size,
            stride,
        };
        self.sender.send(msg).unwrap();
        self.image_key
    }
}

impl binding::WebGpuRenderingContextMethods for WebGpuRenderingContext {
    fn EnumerateAdapters(&self) -> Vec<Root<WebGpuAdapter>> {
        self.adapters.clone()
    }
    fn BuildSwapchain(&self, queue: &WebGpuCommandQueue) -> Root<WebGpuSwapchain> {
        let size = self.canvas.get_size().cast().unwrap();
        let (stride, _) = WebGpuSwapchain::compute_strides(size, 4, queue.get_limits());
        self.canvas_size.set((size, stride as _));
        let frame_count = 3;
        WebGpuSwapchain::new(
            &self.global(),
            self.sender.clone(),
            self.canvas.upcast(),
            self.presenter.clone(),
            queue,
            frame_count,
            WebGpuFormat::B8G8R8A8_SRGB,
            size,
        )
    }
}

pub trait LayoutCanvasWebGpuRenderingContextHelpers {
    #[allow(unsafe_code)]
    unsafe fn canvas_data_source(&self) -> HTMLCanvasDataSource;
}

impl LayoutCanvasWebGpuRenderingContextHelpers for LayoutJS<WebGpuRenderingContext> {
    #[allow(unsafe_code)]
    unsafe fn canvas_data_source(&self) -> HTMLCanvasDataSource {
        HTMLCanvasDataSource::WebGpu((*self.unsafe_get()).layout_handle())
    }
}
