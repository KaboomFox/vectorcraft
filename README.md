# Pictures for pull requests

This branch holds only the screenshots linked from pull requests to
[storytold/vectorcraft](https://github.com/storytold/vectorcraft), so the pictures stay out of the
repository's history. Each folder belongs to one pull request.

- `exporter-plugins/`: exporter plug-ins (file formats written by WebAssembly plug-ins). VectorCraft
  run under a virtual display and driven over its control channel, with a test exporter written in
  WebAssembly text (`vectorcraft_plugins::wat::echo_exporter`).
- `parameter-dialogs/`: labelled, sectioned parameter dialogs for plug-ins, with units, help and
  parameters shown only when they apply. Same setup, with a test filter written in WebAssembly text.
- `live-effect-paints/`: live effects that read the fills and strokes of the object they sit on. A
  test effect that draws a triangle only while the object's stroke is 5 pt wide, shown at 5 pt and
  at 3 pt.
