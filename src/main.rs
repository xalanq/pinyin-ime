extern crate pinyin_ime;

use pinyin_ime::gen;
use pinyin_ime::load::*;
use pinyin_ime::sina;
// use pinyin_ime::eval::PinyinIME;

fn main() {
    let (hanzi_v, hanzi_m) = load_hanzi("./data/hanzi.txt");
    sina::gen_sen("./tmp/raw/sina", "./tmp/text/sina/sina.txt", &hanzi_m);
    gen::gen_word("./tmp/word", "./tmp/word/dict.utf8", "./data/word.txt", &hanzi_m);

    let (word_v, word_m, _) = load_word("./data/word.txt", &hanzi_m);
    gen::gen_jieba_dict("./data/jieba.txt", &hanzi_v, &word_v);
    let jb = load_jieba("./data/jieba.txt");

    gen::gen_gram(
        "./tmp/text/sina",
        "./data/sina",
        (1000000, 1000000, 1000000, 1000000),
        &hanzi_v,
        &hanzi_m,
        &word_v,
        &word_m,
        &jb,
    );
    gen::gen_total_gram("./data");
    // PinyinIME::default();
}
