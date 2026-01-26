# heif

A HEIF image decoder written purely from scratch

# Status

Currently, this crate can parse the ISOBMFF container, extract HEVC parameter sets, and decode image metadata. HEVC slice decoding for actual image reconstruction is still in progress

# Reading

## Coding

https://en.wikipedia.org/wiki/High_Efficiency_Video_Coding<br>
https://en.wikipedia.org/wiki/Exponential-Golomb_coding<br>

<br>

https://www.adobe.com/creativecloud/file-types/image/raster/heic-file.html<br>
https://www.adobe.com/creativecloud/file-types/image/comparison/heic-vs-jpeg.html<br>
