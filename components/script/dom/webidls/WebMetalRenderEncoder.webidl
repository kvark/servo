/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

interface WebMetalRenderEncoder {
    void setRenderPipelineState(WebMetalRenderPipelineState pipeline);
    void drawPrimitives(unsigned long start, unsigned long count, unsigned long instances);
    void endEncoding();
};
