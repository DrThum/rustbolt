# Rustbolt

A World of Warcraft-compatible server emulator, designed to work with the original The Burning Crusade client, version 2.4.3. Mainly used as a Rust learning project.

## Claude - user relationship

*IMPORTANT*: in this project, the user wants to keep writing the code themselves. Claude is there to review the code, give architectural guidance and answer questions about the code base. Unless the user explicitly asks Claude to write code, Claude should not write code except for tests. Drafting plans is still allowed.

## Components

- `auth/`: authentication server
- `shared/`: code that's used in several other components
- `tools/`: various tools used to extract data from the WoW client and explore Rustbolt-specific assets like extracted terrain geometry
- `web_proxy/`: used by the Cartographer tool, exposes a REST API
- `world/`: the world server, handling the world simulation and input/output via network packets from the clients

## IMPORTANT: brainstorm/design sessions

When the user asks for a brainstorming and/or design session, ask questions while co-building the plan. Challenge user's suggestions when you think you have a better one.
During brainstorming sessions, ask the user before digging in the code when in doubt about a previous design or implementation decision.

## IMPORTANT: plan writing

The plans Claude writes will be implemented by a less-capable model so they need to leave no room for interpretation. All decisions must be locked, but the plan itself
must not contain actual code replacement instructions. It SHOULD contain pointers to the files that need modification but no instructions like "replace line 26 with xxx".
The plan MUST include architecture decisions like "the function responsible for doing X must be implemented in file Y".

## Cheap-Worker Delegation Tools (Token Saving)

Three CLI tools delegate bulk I/O to a cheap worker model. Use them to save tokens.

### ask-kimi — bulk reading
Returns a structured summary:

```bash
ask-kimi --paths <file1> <file2>... --question "<specific question>"
```

NEVER read a single file >100 lines yourself, NEVER read 3+ files in a row yourself. In these cases, ALWAYS delegate to `ask-kimi` when you need a summary.

EXCEPTION: if you need a specific line number, read the file yourself.

### extract-chat — chat transcript extraction
Extracts human-readable text from Claude Code JSONL transcripts:

```bash
extract-chat <session.jsonl> -o /tmp/chat.txt
```

ALWAYS use this to edit documentation, including the GDD, after a brainstorming session.

### Documentation workflow (MANDATORY)
**NEVER write documentation directly. Always delegate:**

1. Extract chat: `extract-chat <latest-session.jsonl> -o /tmp/chat.txt`
2. Ask worker to read chat + existing docs and suggest updates:
   `ask-kimi --paths /tmp/chat.txt <doc-files> --question "read chat, give exact changes for docs"`
3. Apply the worker's changes via Edit tool

### When NOT to delegate
- Tasks under ~2000 tokens of work (delegation overhead isn't worth it)
- Brainstorming sessions about game mechanics or content
- When exact line numbers are needed for editing
