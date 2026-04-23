# protoApp docs

These docs follow the [Diátaxis](https://diataxis.fr) framework — four
quadrants, each with a single, distinct purpose.

| Quadrant | When you need to... | Start here |
|---|---|---|
| [Tutorials](./tutorials/) | **learn by doing** — end-to-end walkthroughs for someone new to the project | [Getting started](./tutorials/getting-started.md) |
| [How-to guides](./how-to/) | **solve a specific task** — you already know the basics, just show you the recipe | [Swap the default LLM](./how-to/swap-llm-model.md) |
| [Reference](./reference/) | **look up a fact** — endpoint schemas, feature flags, commands | [OpenAI-compatible API](./reference/openai-api.md) |
| [Explanation](./explanation/) | **understand why** — background, trade-offs, architecture | [Architecture overview](./explanation/architecture.md) |

If you're unsure where something should live, the litmus test is:
- **Learning** → tutorial
- **Goal** → how-to
- **Information** → reference
- **Understanding** → explanation

Do not mix categories in one document. A tutorial is ruined by reference
tables; a reference page is ruined by opinion.
