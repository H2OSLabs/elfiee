## 我做了什么（1）：竞品分析

详情见 docs/mvp/phase2/competitor_analysis_3.md

> 研究问题：为什么现有主流工作流无法直接进化为Elfiee？

Elfiee 要解决的问题分为两个维度
Record：能否记录完整、真实的决策上下文
Learn：AI 能否基于这些记录产生第二次改进
现有工作流在结构上就无法实现这两个维度的闭环
大厂流——管理流
Google 流——文档流
Vibe Coding 流——单次生成流

一句话 takeaway：Elfiee 不是“更好用的工具”，而是更加AI-native的可学习决策工作流的平台产品。


## 我做了什么（2）：竞品分析

详情见 docs/mvp/phase2/competitor_analysis_3.md

> 研究问题：主流“skills”和Elfiee的“What is good”不同在？

主流Skills的问题：
默认skills正确，没有失败历史、强化/削弱机制：
Elfiee 的 Traceability 链条可以根据业务结果自动反过来更新 What is good 的 Strength。
专家手动编写，非共识驱动：
Effiee 可以利用 Consensus History，从团队解决冲突的原始日志中自动“提炼”出属于该团队的 Skill。

一句话 takeaway：我们可以强调 Elfiee 这样的价值——
Skill 不是“定义出来的”，而是“用出来的”；
你的工作过程本身就是 Skill 的生产线：你不需要停下来写规范，Elfiee 就会自动帮助你把任务、会话、实现、验证串起来，沉淀出可复用的项目技能与系统技能。

## 我做了什么（3）：竞品分析

详情见 docs/mvp/phase2/competitor_analysis_3.md

研究问题：其他产品的内外双循环

Elfiee 的两条循环：
内循环（形成与进化 What-is-good 的循环）：意图 / 决策 / Skill（what is good）/ Event / Traceability
外循环（完成项目的循环）：意图 / 代码 / Terminal / 测试 / Repo
调研的产品：
Emacs Org-mode / 其他文学化编程工具 / Claude Code：有内外双循环，两个循环之间需要手动桥接
Plandex / Aider / Cursor / Windsurf / Jira+Github：没有内循环

再次说明：现有工作流在结构上就无法实现这两个维度的闭环。

## 结论（1）：产品价值

见docs/mvp/phase2/Product Value.md

- 动作即资产
    - 你的工作过程本身就是 Skill 的生产线。
    - 你不需要停下来写规范，Elfiee 就会自动帮助你把任务、会话、实现、验证串起来，沉淀出可复用的项目技能与系统技能。
    - 同时这也意味着，你的 Skill 变成了“可评估、可回滚、可迭代”的对象：每条 skill 都能被会话记录、代码变更、测试结果与 Git 提交证据支撑，从而用数据持续校准，而不是凭感觉维护。

## 结论（2）：实验设计

见docs/mvp/phase2/experiment_plan.md

Dogfooding 实验的目标：
验证 First-use 的历史记录是否在 Second-use 中被复用，
并对结果产生了可测量的正向影响。
效率影响 (Time to First Commit, Clarification Count, Rework Count)
准确度影响 (Constraint Miss Count, Directional Error Count, Post-hoc Fix Count)
学习行为变化 (History Reference Count, Skill Reuse / Update)

如果 First-use 的记录存在，但未在 Second-use 中带来上述任一类改进，则我们的设计就是不够合理的。

## 我做了什么（4）：和开发进度对齐

当前开发进度 =《SKILL》+《Elfiee使用指南》

对比《Experiment Plan》和当前开发进度，列出还未满足实验需要的点，并给出手动替代的方案:docs/mvp/phase2/Missing tools and Manual Substitutes.md。
对这些点排优先级，优先开发影响实验进行的。

## 接下来要做什么——Dogfooding

见docs/mvp/phase2/Dogfooding Plan.md