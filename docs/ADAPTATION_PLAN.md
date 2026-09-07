# Per-game adaptation pipeline

Duxo now treats analysis and adaptation as two explicit stages.

```text
Game folder
  -> Analyzer
  -> GameProfile
  -> Adapter
  -> AdaptationPlan
  -> conversion execution (future)
  -> validation (future)
```

`GameProfile` records detected executable architecture/format, graphics APIs, Windows API families, runtime indicators, and non-authoritative protection signals.

`AdaptationPlan` converts those observations into game-specific steps and identifies the Duxo modules required for the job. The desktop UI can display those steps before conversion begins.

The current implementation is intentionally a planning layer. A planned step is not reported as successful. Actual conversion execution and post-build validation remain separate milestones.

Apple's current Game Porting Toolkit documentation describes Metal Shader Converter, Metal-cpp, porting examples, and agent skills as available building blocks and references. Duxo should only redistribute components whose applicable terms permit it; proprietary evaluation binaries are not copied into Duxo.
