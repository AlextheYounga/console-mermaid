# Agent Onboarding

You are a **Copilot agent**.  

## How to Work

You do not write code until we discuss the reasoning behind your change first. You should begin. You should draft a quick summary of your intentions and thoughts, and if I agree, then you should proceed.


## Engineering Philosophy

The following defines our **engineering culture, preferences, and non-negotiables**.  
Agents should reference this before making any design or implementation decisions.

---

## Principles

- **Minimize assumptions.** Validate ideas with code or tests instead of speculation.
- **Work incrementally.** Ship in small, verifiable steps.
- **Get it running first.** Deliver working code before refactoring.
- **Follow conventions.** Always check existing files before inventing a new style.
- **Avoid premature abstraction.** Don’t generalize until there’s a proven need.
- **Ask: _What would John Carmack do?_** Optimize for clarity, correctness, and performance.

---

## Opinions

- **Simplicity first:** Complexity is a liability.
- **Comments are code:** Do not write excessive comments in the same way you would not write excessive code.
- **KISS > DRY:** Clarity (Keep It Simple, Stupid) is usually better than deduplicating everything. DRY should not create indirection.
- **CLEAN CODE:** Clean Code principles are non-negotiable. “Clear code is clear thinking.”
- **Consistency is king:** Check other files for conventions and for existing solutions to inspire future solutions.
- **FOSS > SaaS:** Never use closed-source third-party services if open-source exists. We are self-hosting maximalists.
- **Leverage before adding:** Always ask: _“Can existing tools (like SQLite) solve this before adding Redis/new dependencies?”_
- **Idempotency:** We like idempotent setups.
- **YAGNI:** (You Aren’t Gonna Need It). Don’t add layers for problems we don’t yet have.
- **Cohesion > Coupling:** Related things belong together; unrelated things must be separated.
- **Transparency > Magic:** Avoid “clever” metaprogramming or hidden behaviors; explicitness wins. Ruby often upsets us because of this.
- **Avoid hack solutions:** Consider alternative approaches instead of applying patchwork solutions to poorly-thought implementations. Example: never wrangle with strings (regexing, manipulating for comparisons, etc) if you don't have to.

---

