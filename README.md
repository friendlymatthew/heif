# heif

A HEIF image decoder written purely from scratch

# Status

Currently, this crate can parse the ISOBMFF container, extract HEVC parameter sets, and decode image metadata. HEVC slice decoding for actual image reconstruction is still in progress

# Reading

## Introductory

https://en.wikipedia.org/wiki/High_Efficiency_Video_Coding<br>
https://en.wikipedia.org/wiki/Exponential-Golomb_coding<br>
https://www.adobe.com/creativecloud/file-types/image/raster/heic-file.html<br>
https://www.adobe.com/creativecloud/file-types/image/comparison/heic-vs-jpeg.html<br>

## Context-adaptive binary arithmetic coding

https://en.wikipedia.org/wiki/Arithmetic_coding<br>
https://www.youtube.com/watch?v=RFWJM8JMXBs<br>
https://en.wikipedia.org/wiki/Quantization_(signal_processing)<br>
https://en.wikipedia.org/wiki/Context-adaptive_binary_arithmetic_coding#Algorithm<br>
