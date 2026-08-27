# Synthetic MV/MZ fixtures

These tiny games are original test data committed with Hoshi2Star. They contain
no files copied from a commercial or personal game.

- `mv/www/data` locks the RPG Maker MV directory layout and four-parameter
  `Show Text` header.
- `mz/data` locks the RPG Maker MZ directory layout, speaker parameter and
  supported `TextPicture` command.

Both fixtures exercise static database text, dialogue, choices, scrolling text,
branches, engine variables and deterministic pilot sampling. Keep them small:
large or private games belong to optional manual pilot tests, not CI.
