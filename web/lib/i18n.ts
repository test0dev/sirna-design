export type Locale = "zh" | "en";

export const LOCALE_STORAGE_KEY = "sirna-locale";
export const DEFAULT_LOCALE: Locale = "en";

type Vars = Record<string, string | number>;

const zh = {
  brand: "siRNA 设计",
  language: "语言",
  langZh: "中文",
  langEn: "EN",
  newDesign: "新建设计",

  designTitle: "设计 siRNA",
  designLead:
    "输入基因符号自动解析 MANE Select 转录本，或粘贴核苷酸序列。选项沿用 siDirect 2.1：功能规则、seed Tm，再做转录本特异性检查。",
  fastaOrPlain: "FASTA 或纯序列",
  upToNt: "最长 {max} nt",

  targetInput: "目标输入",
  sequence: "序列",
  sequenceHelp:
    "先输入基因符号解析 MANE Select（得到 NM_），也可直接用 accession 检索或粘贴序列。",
  loadSample: "载入样本",
  geneSymbol: "基因符号",
  resolveAndLoad: "解析并载入",
  resolvingTarget: "正在解析目标…",
  accessionNumber: "Accession 编号",
  retrieveSequence: "检索序列",
  loadedSample: "已载入 CLDN17 样本转录本。",
  loadedTranscript: "已载入 {header} · {length} nt。",
  resolveSummary:
    "{symbol} · {transcript} · {refseq} · {tag}",
  maneSelectTag: "MANE Select",
  canonicalTag: "canonical",
  errGeneEmpty: "请先输入基因符号。",
  errGeneNotFound: "未找到基因：{symbol}。",
  errResolveFailed: "目标解析失败。",
  errAccessionEmpty: "请先输入 accession 编号。",
  errRetrieveFailed: "序列检索失败。",
  errDesignFailed: "设计请求失败。",
  errAccessionNotFound: "未找到该 accession：{id}。",
  invalidChars: "非法字符：{chars}",
  exceedsLimit: "超过 10 kbp 上限",

  functionalSelection: "功能筛选",
  efficacyAlgorithms: "功效算法",
  efficacyHelp: "Ui-Tei 是主筛选。Reynolds 与 Amarzguioui 可用 +（并集）或 ×（交集）组合。",
  on: "开",
  off: "关",
  combinedRule: "组合规则",
  combinedRuleHelp: "+ 保留任一算法通过的候选。× 要求列出的算法全部通过。",
  combineHintUnion: "并集 — 保留通过任一所选算法的候选",
  combineHintMixed: "保留通过（Ui-Tei 或 Reynolds）并且通过 Amarzguioui 的候选",
  combineHintAll: "交集 — 保留通过全部所选算法的候选",

  offTarget: "脱靶",
  seedDuplex: "Seed 双链稳定性",
  seedDuplexHelp: "较低的 seed Tm 可降低引导链和乘客链上的 miRNA 式脱靶沉默。",
  maxTm: "最高 Tm",
  seedTmHelp:
    "siDirect 将 21.5°C 作为引导链 2–8 位 seed-target 双链 Tm 的初始上限。高于该值的候选更容易产生 seed 依赖脱靶。",
  recommended: "推荐",
  seedRegion: "Seed 区域",
  guide28: "引导链 2–8",
  checkedStrands: "检查链",
  guideAndPassenger: "引导链与乘客链",

  specificity: "特异性",
  transcriptDb: "转录本数据库",
  transcriptDbHelp: "在两条链第 2–20 位的 19-mer 上搜索近乎完全匹配。",
  specificityCheck: "特异性检查",
  specNone: "无",
  specHuman: "人 (Homo sapiens) 转录本",
  specMouse: "小鼠 (Mus musculus) 转录本",
  specRat: "大鼠 (Rattus norvegicus) 转录本",
  hideLessSpecific: "隐藏低特异性 siRNA",
  hideLessSpecificHint: "siDirect 默认：与任何非目标转录本至少有两个错配。",
  showOffTargetHits: "显示三个错配以内的脱靶命中数",
  showOffTargetHitsHint: "统计每个候选的 19/19、18/19、17/19、16/19 命中。",

  constraints: "约束",
  otherOptions: "其他选项",
  otherOptionsHelp: "功能规则之后的可选区间、组成和基序过滤。",
  targetRange: "靶向区间",
  gcContent: "GC 含量",
  avoidGC: "避免连续 G 或 C",
  avoidGCHint: "适用于化学合成 siRNA",
  avoidAT: "避免连续 A 或 T",
  avoidATHint: "适用于带 pol III 启动子的 shRNA 载体",
  ntOrMore: "nt 及以上",
  customPattern: "自定义模式",
  customPatternHelp:
    "使用 IUPAC 代码，例如 N 表示任意碱基。siDirect 的 shRNA 示例 NNG… 将目标第三位固定为 G。",
  excludePattern: "排除模式",
  excludePatternHelp: "丢弃目标序列包含该基序的候选。",
  matchAll: "只显示同时满足全部已选条件的 siRNA",
  matchAllHint: "组合过严导致没有结果时，可关闭此项。",
  submitHint: "经本地代理提交到 siDirect，再将 HTML 解析为结果 JSON。",
  designSirna: "设计 siRNA",

  resultsTitle: "{symbol} siRNA 设计结果",
  resultsLead: "{name}。候选位点已映射到所提供的转录本，并按当前设计规则评估。",
  candidates: "候选",
  passAllRules: "通过全部规则",
  bestScore: "最高得分",

  inputContext: "输入上下文",
  designParameters: "设计参数",
  valuesFromJson: "来自本次设计提交",
  combinedRuleLabel: "组合规则",
  seedTmCeiling: "Seed Tm 上限",
  duplex: "双链",
  duplexValue: "{length} nt + {overhang} nt 突出端",
  avoidContiguous: "避免连续碱基",
  specificityFilters: "特异性过滤",
  hideLessSpecificShort: "隐藏低特异性",
  showAll: "显示全部",
  showHits: "显示脱靶命中",
  hideHits: "隐藏脱靶命中",
  enabledAlgorithms: "已启用算法",

  rankedOutput: "排序输出",
  effectiveCandidates: "有效 siRNA 候选",
  selectRow: "选择一行以在转录本上定位该候选。",
  candidatesCount: "{count} 条候选",
  position: "位置",
  targetSequence: "靶序列",
  guide53: "引导链 (5′→3′)",
  rules: "规则",
  score: "得分",
  selected: "已选",

  sequenceContext: "序列上下文",
  graphicalView: "有效 siRNA 候选的图形视图",
  wrapHelp: "转录本每 {n} nt 换行。重叠候选分车道堆叠。",
  seedDuplexTm: "Seed 双链 Tm",
  functionalSirna: "功能性 siRNA",
  offTargetReduced: "脱靶降低",
  collapsedEmptyGap: "已折叠 {rows} 行无候选 · {start}–{end}（{nt} nt）",
  expandEmptyGap: "展开",
  collapseEmptyGap: "收起无候选区域",

  passed: "通过",
  review: "复查",
  targetSense: "靶标 / 正义链",
  guideAntisense: "引导 / 反义链",
  designRules: "设计规则",

  loadingResults: "正在加载结果…",
} as const;

const en: Record<keyof typeof zh, string> = {
  brand: "siRNA Design",
  language: "Language",
  langZh: "中文",
  langEn: "EN",
  newDesign: "New design",

  designTitle: "Design siRNA",
  designLead:
    "Enter a gene symbol to resolve the MANE Select transcript, or paste a nucleotide sequence. Options follow the siDirect 2.1 workflow: functional rules, seed Tm, then transcript specificity.",
  fastaOrPlain: "FASTA or plain sequence",
  upToNt: "up to {max} nt",

  targetInput: "Target input",
  sequence: "Sequence",
  sequenceHelp:
    "Start with a gene symbol to resolve MANE Select (NM_), or retrieve by accession / paste a sequence.",
  loadSample: "Load sample",
  geneSymbol: "Gene symbol",
  resolveAndLoad: "Resolve and load",
  resolvingTarget: "Resolving target…",
  accessionNumber: "Accession number",
  retrieveSequence: "Retrieve sequence",
  loadedSample: "Loaded the CLDN17 sample transcript.",
  loadedTranscript: "Loaded {header} · {length} nt.",
  resolveSummary: "{symbol} · {transcript} · {refseq} · {tag}",
  maneSelectTag: "MANE Select",
  canonicalTag: "canonical",
  errGeneEmpty: "Enter a gene symbol first.",
  errGeneNotFound: "Gene not found: {symbol}.",
  errResolveFailed: "Target resolution failed.",
  errAccessionEmpty: "Enter an accession number first.",
  errRetrieveFailed: "Sequence retrieval failed.",
  errDesignFailed: "Design request failed.",
  errAccessionNotFound: "Accession not found: {id}.",
  invalidChars: "Invalid: {chars}",
  exceedsLimit: "Exceeds 10 kbp limit",

  functionalSelection: "Functional selection",
  efficacyAlgorithms: "Efficacy algorithms",
  efficacyHelp:
    "Ui-Tei is the primary filter. Reynolds and Amarzguioui can be combined with + (OR) or × (AND).",
  on: "On",
  off: "Off",
  combinedRule: "Combined rule",
  combinedRuleHelp:
    "+ keeps the union of algorithms. × requires every listed algorithm to pass.",
  combineHintUnion: "Union — keep candidates that pass any selected algorithm",
  combineHintMixed:
    "Keep candidates that pass (Ui-Tei or Reynolds) and Amarzguioui",
  combineHintAll: "Intersection — keep candidates that pass every selected algorithm",

  offTarget: "Off-target",
  seedDuplex: "Seed-duplex stability",
  seedDuplexHelp:
    "Lower seed Tm reduces miRNA-like off-target silencing on both guide and passenger strands.",
  maxTm: "Max Tm",
  seedTmHelp:
    "siDirect uses 21.5°C as the initial ceiling for seed-target duplex Tm at guide positions 2–8. Candidates above this threshold are more likely to cause seed-dependent off-target effects.",
  recommended: "Recommended",
  seedRegion: "Seed region",
  guide28: "Guide 2–8",
  checkedStrands: "Checked strands",
  guideAndPassenger: "Guide and passenger",

  specificity: "Specificity",
  transcriptDb: "Transcript database",
  transcriptDbHelp:
    "Near-perfect matches are searched on 19-mers from positions 2–20 of both strands.",
  specificityCheck: "Specificity check",
  specNone: "None",
  specHuman: "Human (Homo sapiens) transcript",
  specMouse: "Mouse (Mus musculus) transcript",
  specRat: "Rat (Rattus norvegicus) transcript",
  hideLessSpecific: "Hide less-specific siRNAs",
  hideLessSpecificHint:
    "Default siDirect filter: at least two mismatches to any non-target transcript.",
  showOffTargetHits: "Show off-target hits within three mismatches",
  showOffTargetHitsHint: "Count 19/19, 18/19, 17/19 and 16/19 hits for each candidate.",

  constraints: "Constraints",
  otherOptions: "Other options",
  otherOptionsHelp:
    "Optional range, composition, and motif filters applied after the functional rules.",
  targetRange: "Target range",
  gcContent: "GC content",
  avoidGC: "Avoid contiguous G's or C's",
  avoidGCHint: "For chemically synthesized siRNA",
  avoidAT: "Avoid contiguous A's or T's",
  avoidATHint: "For shRNA vectors with a pol III promoter",
  ntOrMore: "nt or more",
  customPattern: "Custom pattern",
  customPatternHelp:
    "Use IUPAC codes such as N for any base. The siDirect shRNA example NNG… fixes the third target base to G.",
  excludePattern: "Exclude pattern",
  excludePatternHelp: "Discard candidates whose target sequence contains this motif.",
  matchAll: "Only show siRNAs that match all checked criteria",
  matchAllHint: "Turn this off if a strict combination returns no candidates.",
  submitHint:
    "Submits through a local proxy to siDirect, then parses HTML into the result JSON.",
  designSirna: "Design siRNA",

  resultsTitle: "{symbol} siRNA design results",
  resultsLead:
    "{name}. Candidate sites are mapped against the supplied transcript and evaluated with the configured design rules.",
  candidates: "Candidates",
  passAllRules: "Pass all rules",
  bestScore: "Best score",

  inputContext: "Input context",
  designParameters: "Design parameters",
  valuesFromJson: "Values supplied with this design",
  combinedRuleLabel: "Combined rule",
  seedTmCeiling: "Seed Tm ceiling",
  duplex: "Duplex",
  duplexValue: "{length} nt + {overhang} nt overhang",
  avoidContiguous: "Avoid contiguous bases",
  specificityFilters: "Specificity filters",
  hideLessSpecificShort: "Hide less-specific",
  showAll: "Show all",
  showHits: "Show off-target hits",
  hideHits: "Hide off-target hits",
  enabledAlgorithms: "Enabled algorithms",

  rankedOutput: "Ranked output",
  effectiveCandidates: "Effective siRNA candidates",
  selectRow: "Select a row to locate the candidate on the transcript.",
  candidatesCount: "{count} candidates",
  position: "Position",
  targetSequence: "Target sequence",
  guide53: "Guide (5′→3′)",
  rules: "Rules",
  score: "Score",
  selected: "Selected",

  sequenceContext: "Sequence context",
  graphicalView: "Graphical view of effective siRNA candidates",
  wrapHelp:
    "Transcript is wrapped every {n} nt. Overlapping candidates are stacked on separate lanes.",
  seedDuplexTm: "Seed duplex Tm",
  functionalSirna: "Functional siRNA",
  offTargetReduced: "off-target reduced",
  collapsedEmptyGap: "Collapsed {rows} empty rows · {start}–{end} ({nt} nt)",
  expandEmptyGap: "Expand",
  collapseEmptyGap: "Collapse empty region",

  passed: "Passed",
  review: "Review",
  targetSense: "Target / sense",
  guideAntisense: "Guide / antisense",
  designRules: "Design rules",

  loadingResults: "Loading results…",
};

const dictionaries: Record<Locale, Record<keyof typeof zh, string>> = { zh, en };

export type MessageKey = keyof typeof zh;

export function isLocale(value: string | null): value is Locale {
  return value === "zh" || value === "en";
}

export function translate(
  locale: Locale,
  key: MessageKey,
  vars?: Vars,
): string {
  const template = dictionaries[locale][key] ?? dictionaries.en[key] ?? key;
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (_, name: string) =>
    String(vars[name] ?? ""),
  );
}

export function combineHintKey(
  value: string,
): Extract<MessageKey, "combineHintUnion" | "combineHintMixed" | "combineHintAll"> {
  if (value.includes("×") && value.indexOf("+") === -1) return "combineHintAll";
  if (value.includes("×")) return "combineHintMixed";
  return "combineHintUnion";
}

export function specificityOptionLabel(
  value: string,
  englishLabel: string,
  t: (key: MessageKey) => string,
): string {
  if (value === "none") return t("specNone");
  const species = value.startsWith("human")
    ? t("specHuman")
    : value.startsWith("mouse")
      ? t("specMouse")
      : t("specRat");
  const release = englishLabel.split("transcript, ")[1] ?? "";
  return release ? `${species}, ${release}` : species;
}

export function localizeThrownMessage(
  message: string,
  t: (key: MessageKey, vars?: Vars) => string,
): string {
  if (message === "Enter a gene symbol first.") return t("errGeneEmpty");
  if (message === "Enter an accession number first.") return t("errAccessionEmpty");
  if (message === "Sequence retrieval failed.") return t("errRetrieveFailed");
  if (message === "Design request failed.") return t("errDesignFailed");
  if (message === "Target resolution failed.") return t("errResolveFailed");
  const geneNotFound = message.match(/^Gene not found:\s*(.+?)\.?$/i);
  if (geneNotFound) return t("errGeneNotFound", { symbol: geneNotFound[1] });
  const notFound = message.match(/^Accession not found:\s*(.+?)\.?$/);
  if (notFound) return t("errAccessionNotFound", { id: notFound[1] });
  return message;
}
