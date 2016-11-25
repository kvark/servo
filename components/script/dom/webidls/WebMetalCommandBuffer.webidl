/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

dictionary RenderTarget {
    WebMetalTargetView view;
    sequence<float> clear;
};

dictionary RenderTargetSet {
    RenderTarget color0;
    RenderTarget color1;
    RenderTarget color2;
    RenderTarget color3;
    RenderTarget depthStencil;
};

interface WebMetalCommandBuffer {
    WebMetalRenderEncoder   makeRenderEncoder(optional RenderTargetSet targets);
};
