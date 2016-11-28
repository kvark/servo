/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

dictionary RenderPipelineDesc {
    required DOMString vertexFunction;
    required DOMString fragmentFunction;
};

interface WebMetalDevice {
    WebMetalBuffer                  makeBuffer(MetalSize size, object? data);
    WebMetalRenderPipelineState     makeRenderPipelineState(optional RenderPipelineDesc desc);

//    WebMetalTexture                 makeTexture();
//  WebMetalFence                   makeFence();
//    WebMetalSamplerState            makeSamplerState();
//  WebMetalComputePipelineState    makeComputePipelineState();
};
