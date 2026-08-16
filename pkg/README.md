## To be named

A basic PBR path tracer following the ideas from learn-wgpu, the new gpu-tracing book, and many other resources on the web. My goal is eventually to make it rigorous after completing PBRTv4.

## Gallery:
Excuse the limited capability for now

[Car model](https://sketchfab.com/3d-models/ferrari-296-gt3-verstappen-wwwvecarzcom-cd9e1436a1a2471ea4106490f2ec8955) by [vecarz](https://sketchfab.com/heynic) and [MattDoesBlender](https://sketchfab.com/MattDoesBlender), used under [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/)

![Render](/pics/render1.png)

## Notes
Todos:
- [x] BVH SAH sweep + data via storage buffer
- [x] Cook-Torrance BRDF + importance sampling the NDF
- [x] glTF model loading + material integration
- [x] Texture support for the web (Atlas?)
- [ ] some sort of ui (egui), scenes, etc.
- [ ] clean and restructure
- [ ] upgrade it to a BSDF (read the paper + pbrtv4 9.3-9.7)

less priority
- [ ] orbit/pan cam
- [ ] MIS for light sources
- [ ] volumetrics/clouds
- [ ] research (https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos)

Turns out that this whole PBR thingy is a MASSIVE rabbit hole.
Anyways, resources in no particular order:

- https://agraphicsguynotes.com/posts/sample_microfacet_brdf/ -> same as below
- https://schuttejoe.github.io/post/ggximportancesamplingpart1/ -> importance for NDF
- https://lisyarus.github.io/blog/posts/multiple-importance-sampling.html -> MIS
- https://jacco.ompf2.com/2022/04/18/how-to-build-a-bvh-part-2-faster-rays/ -> SAH
- https://google.github.io/filament/Filament.md.html -> Different forms of the geometry function
- https://www.youtube.com/watch?v=gya7x9H3mV0 -> PBR foundations
- https://www.youtube.com/watch?v=j-A0mwsJRmk&t=842s
- https://www.pbr-book.org/4ed/Reflection_Models -> urgent
- https://cseweb.ucsd.edu/~tzli/cse272/wi2026/ - goldmine

Future reads:
- https://jcgt.org/published/0003/02/03/paper.pdf
- https://www.cs.cornell.edu/~srm/publications/EGSR07-btdf.pdf
- https://jcgt.org/published/0007/04/01/paper.pdf -> VNDF
