# SkillHub CLI

Planned Rust CLI client (`skillhub` binary):

```
skillhub login --token sk_xxx --registry https://skill.example.com
skillhub search pdf
skillhub install pdf-parser --agent codex
skillhub publish ./my-skill --slug my-space--my-skill --version 1.0.0
skillhub list
```

Implementation pending — first iteration may live as a separate crate
in this workspace or as a thin shell around the REST API.
