extern crate pinyin_ime;
use std::env;

/*
use pinyin_ime::gen;
use pinyin_ime::load::*;
use pinyin_ime::sina;
*/
use pinyin_ime::eval::PinyinIME;

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
    /*
    let seq = [
        "qing hua da xue ji suan ji xi",
        "ren gong zhi neng",
        "ji qi xue xi",
        "shu ju wa jue",
        "qing wen gou li guo jia sheng si yi qi yin huo fu bi qu zhi shi shen me yi si",
        "san yue mo qing hua da xue cheng li le yi ge xin de yuan xi tian wen xi",
        "wo men yao qiu tong xue men zi ji bian cheng shi xian yi ge jian dan de han yu pin yin shu ru fa",
        "ni men de shui ping ke zhen shi cen ci bu qi ne",
        "zhe ge dong xi wo bang ni men zuo hao le",
        "ni hao sao a",
        "xi jin ping dui e luo si jin xing guo shi fang wen",
        "pu jing zai ji chang qin zi ying jie xi jin ping",
        "jin tian shang wu wai jiao bu zhao kai xin wen fa bu hui wang yi bu zhang qin zi fa biao jiang hua",
        "yu ci tong shi guo wu yuan ye zhao kai xin wen fa bu hui li ke qiang zong li ye qin zi fa biao jiang hua",
        "lao shi gei de yu liao ku zhen de shi you hong you zhuan",
        "you ren you bi jiao tie jin sheng huo de yu liao ku ma bi ru shuo zhi hu bi li bi li",
    ];
    let ans = [
        "清华大学计算机系",
        "人工智能",
        "机器学习",
        "数据挖掘",
        "请问苟利国家生死以岂因祸福避趋之是什么意思",
        "三月末清华大学成立了一个新的院系天文系",
        "我们要求同学们自己编程实现一个简单的汉语拼音输入法",
        "你们的水平可真是参差不齐呢",
        "这个东西我帮你们做好了",
        "你好骚啊",
        "习近平对俄罗斯进行国事访问",
        "普京在机场亲自迎接习近平",
        "今天上午外交部召开新闻发布会王毅部长亲自发表讲话",
        "与此同时国务院也召开新闻发布会李克强总理也亲自发表讲话",
        "老师给的语料库真的是又红又专",
        "有人有比较贴近生活的语料库吗比如说知乎哔哩哔哩",
    ];
    let seq: Vec<_> = seq.iter().map(|x| x.to_string()).collect();
    let ans: Vec<_> = ans.iter().map(|x| x.to_string()).collect();
    let mut best_pred = Vec::new();

    let ime = PinyinIME::new();
    for i in 0..ans.len() {
        let preds = ime.evals(&seq[i], 5);
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
    */
    let ime = PinyinIME::new("./data/config.json");
    match env::args().len() {
        3 => {
            let input = env::args().nth(1).unwrap();
            let output = env::args().nth(2).unwrap();
            ime.eval_from_only(&input, &output);
        }
        4 => {
            let input = env::args().nth(1).unwrap();
            let output = env::args().nth(2).unwrap();
            let answer = env::args().nth(3).unwrap();
            ime.eval_from(&input, &answer, &output);
        }
        2 => {
            let k = env::args()
                .nth(1)
                .unwrap()
                .parse::<usize>()
                .expect("你需要输入一个整数，表示输出的答案个数");
            if k == 0 {
                println!("参数必须大于0");
            } else {
                println!("将会显示 {} 个解以及概率", k);
                println!("请尽情输入拼音: ");
                let mut input = String::new();
                while let Ok(_) = std::io::stdin().read_line(&mut input) {
                    if input.len() == 0 {
                        break;
                    }
                    let preds = ime.evals(&input, k);
                    if preds.len() == 0 {
                        println!("Error: 无法识别");
                    } else {
                        print!(
                            "{}",
                            preds.iter().fold(&mut String::new(), |s, t| {
                                if s.len() > 0 {
                                    s.push_str(&format!("{} {}\n", t.0, t.1));
                                } else {
                                    s.push_str(&format!("{} {}\n", t.0, t.1));
                                }
                                s
                            })
                        );
                    }
                    input.clear();
                }
                println!("Exit. Good luck!");
            }
        }
        _ => {
            println!("请尽情输入拼音: ");
            let mut input = String::new();
            while let Ok(_) = std::io::stdin().read_line(&mut input) {
                if input.len() == 0 {
                    break;
                }
                match ime.eval(&input) {
                    Some(ans) => println!("{}", ans.0),
                    None => println!("Error: 无法识别"),
                };
                input.clear();
            }
            println!("Exit. Good luck!");
        }
    };
}
