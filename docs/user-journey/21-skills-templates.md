
## User Journey Index（Skill 模板）

| Journey ID  | 角色        | 场景描述           | 结果               |
| ----------- | --------- | -------------- | ---------------- |
| UJ-SKILL-01 | System    | 加载 Skill 模板    | Skill 被 Agent 识别 |
| UJ-SKILL-02 | User / AI | 理解 SKILL.md 结构 | 能力边界清晰           |


---

## 4. 详细 User Journey

### UJ-SKILL-01｜加载 Skill 模板（能力发现）

**场景** Agent 被启用后，系统需要为其加载预定义的能力模板，使 Agent 具备"知道自己能做什么"的认知。

**场景流程**

- 系统在 Agent 启用时扫描 `.elf/Agents/elfiee-client/` 目录。

- 系统识别目录下的 SKILL.md 和 mcp.json 文件对。

- 系统解析 Skill 定义，将其注册为 Agent 的可用能力集合。

- 若目录中存在多个 Skill，系统依次加载并建立能力索引。

**预期结果**

- **能力自动发现：** Agent 不需要人类逐一告知"你能做什么"，而是通过 Skill 模板自动获得能力清单。

- Skill 的加载过程是确定性的：相同的模板在任何项目中都产生相同的基础能力。

---

### UJ-SKILL-02｜SKILL.md 定义能力边界

**场景** Agent 或人类需要理解某个 Skill 的具体能力范围、行为规范与使用约束。

**场景流程**

- 用户或 Agent 读取 SKILL.md 文件。

- SKILL.md 包含结构化的能力描述：Skill 的名称与用途、适用的 Block 类型与操作范围、行为规范与禁止操作清单、输入输出的语义约定。

- Agent 基于 SKILL.md 的描述构建自己的行为边界认知。

- 人类可通过阅读 SKILL.md 审计 Agent 被授予的能力范围。

**预期结果**

- **双向可读性：** SKILL.md 同时服务于 Agent（机器可解析）和人类（自然语言可理解），是能力契约的单一来源。

- Agent 的行为边界不再依赖隐式假设，而是由显式文档定义。
