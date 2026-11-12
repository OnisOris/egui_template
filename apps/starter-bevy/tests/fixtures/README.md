# Owned model fixture

`box.glb` is a procedurally generated box created for this project's tests, with
no third-party model, image, or texture content. Its root translation is
`(4, 8, 0)`; its child's scale is `(2, 3, 1)`. Unit-box vertices span `[-1, 1]`.
The resulting bounds are `(2, 5, -1)` to `(6, 11, 1)`.

The intentionally displaced pivot tests descendant transforms and base anchoring.
The file embeds its indexed triangle geometry and a simple blue PBR material.

`orange-box.glb` is a second owned variant with an orange material and child
scale `(1, 2, 1)`. It checks loading distinct files alongside shared instances.
