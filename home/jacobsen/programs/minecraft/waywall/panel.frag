// Flat colour over every character cell, ignoring the glyph in the font atlas.
//
// The alpha comes from the colour the text object was created with, so
// "#RRGGBBAA" in init.lua controls how much of the game shows through.

precision highp float;

varying vec4 f_dst_rgba;

void main() {
    gl_FragColor = f_dst_rgba;
}

// vim:ft=glsl
