## Pinyin IME - 拼音输入法

基于朴素贝叶斯算法的拼音输入法。同时还支持语料库的生成，n-gram数据的生成。

## Feature

* 拼音生成文字（不支持连在一串的拼音序列，必须人工分割开且与字的拼音一致）
* 分词（[jieba_rs](https://github.com/messense/jieba-rs)）
* 拼音标注（[rust-pinyin](https://github.com/mozillazg/rust-pinyin)、[open-gram的已标注数据](http://yongsun.me/2010/03/open-gram%E9%A1%B9%E7%9B%AE%E7%AE%80%E4%BB%8B/)）
* 基于词与拼音的1~4-gram数据生成
* 不同语料库的加权

## Usage
