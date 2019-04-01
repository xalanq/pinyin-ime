extern crate pinyin_ime;

/*
use pinyin_ime::gen;
use pinyin_ime::load::*;
use pinyin_ime::sina;
*/
use pinyin_ime::eval::{self, PinyinIME};

fn main() {
    /*
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
    */
    let seq = [
        "gou li guo jia sheng si yi",
        "qi yin huo fu bi qu zhi",
        "qing hua da xue ji suan ji xi",
        "ren gong zhi neng",
        "ji qi xue xi",
        "shu ju wa jue",
        "wo men yao qiu tong xue men zi ji bian cheng shi xian yi ge jian dan de han yu pin yin shu ru fa",
        "ni men de shui ping ke zhen shi cen ci bu qi ne",
        "zhe ge dong xi wo bang ni men zuo hao le",
    ];
    let ans = [
        "苟利国家生死以",
        "岂因祸福避趋之",
        "清华大学计算机系",
        "人工智能",
        "机器学习",
        "数据挖掘",
        "我们要求同学们自己编程实现一个简单的汉语拼音输入法",
        "你们的水平可真是参差不齐呢",
        "这个东西我帮你们做好了",
    ];
    let seq: Vec<_> = seq.iter().map(|x| x.to_string()).collect();
    let ans: Vec<_> = ans.iter().map(|x| x.to_string()).collect();
    let mut pred = Vec::new();

    let ime = PinyinIME::default();
    for i in 0..ans.len() {
        pred.push(ime.eval(&seq[i]));
        println!("input: {}\n...predict: {}\n...answer:  {}", seq[i], pred[i], ans[i]);
        let (acc, f1) = eval::score(&ans[i], &pred[i]);
        println!("...acc: {}, f1: {:.2}", acc, f1);
    }
    let (acc, f1) = eval::score_list(&ans, &pred);
    println!("Total acc: {:.2}, f1: {:.2}", acc, f1);
    while true {}
}
