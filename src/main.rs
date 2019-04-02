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
    gen::gen_word("./tmp/word", "./tmp/word/dict.utf8", "./data/word.txt", &hanzi_m, 7);

    let (word_v, word_m, pinyin_m) = load_word("./data/word.txt", &hanzi_m);
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
        &pinyin_m,
        &jb,
    );
    gen::gen_total_gram("./data");
    */
    let seq = [
        "qing hua da xue ji suan ji xi",
        "ren gong zhi neng",
        "ji qi xue xi",
        "shu ju wa jue",
        "qing wen gou li guo jia sheng si yi qi yin huo fu bi qu zhi shi shen me yi si",
        "ni zhi dao chuang qian ming yue guang di shang xie liang shuang ma",
        "wo men yao qiu tong xue men zi ji bian cheng shi xian yi ge jian dan de han yu pin yin shu ru fa",
        "ni men de shui ping ke zhen shi cen ci bu qi ne",
        "zhe ge dong xi wo bang ni men zuo hao le",
        "ni hao sao a",
    ];
    let ans = [
        "清华大学计算机系",
        "人工智能",
        "机器学习",
        "数据挖掘",
        "请问苟利国家生死以岂因祸福避趋之是什么意思",
        "你知道床前明月光地上鞋两双吗",
        "我们要求同学们自己编程实现一个简单的汉语拼音输入法",
        "你们的水平可真是参差不齐呢",
        "这个东西我帮你们做好了",
        "你好骚啊",
    ];
    let seq: Vec<_> = seq.iter().map(|x| x.to_string()).collect();
    let ans: Vec<_> = ans.iter().map(|x| x.to_string()).collect();
    let mut best_pred = Vec::new();

    let ime = PinyinIME::default();
    for i in 0..ans.len() {
        let preds = ime.evals(&seq[i], 3);
        best_pred.push(preds[0].0.clone());
        print!(
            "input: {}\n...answer:  {}\n...predict: {}",
            seq[i],
            ans[i],
            preds.iter().fold(&mut String::new(), |s, t| {
                if s.len() > 0 {
                    s.push_str(&format!("            {} {}\n", t.0, t.1));
                } else {
                    s.push_str(&format!("{} {}\n", t.0, t.1));
                }
                s
            })
        );
        let (acc, f1) = eval::score(&ans[i], &best_pred[i]);
        println!("...acc: {}, f1: {:.2}", acc, f1);
    }
    let (acc, f1) = eval::score_list(&ans, &best_pred);
    println!("Total acc: {:.2}, f1: {:.2}", acc, f1);
}
