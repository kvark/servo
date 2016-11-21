/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

dictionary RenderTargets {
    WebMetalTargetView? color0 = null;
    WebMetalTargetView? color1 = null;
    WebMetalTargetView? color2 = null;
    WebMetalTargetView? color3 = null;
    WebMetalTargetView? depth = null;
    WebMetalTargetView? stencil = null;
};

interface WebMetalCommandBuffer {
    WebMetalRenderEncoder   makeRenderEncoder(optional RenderTargets targets);
};
