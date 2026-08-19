# Personality schema contract

`Personality` is an editable, bounded descriptive profile. It is not an instruction
layer and cannot override security, project or policy controls. Unknown fields are
rejected during deserialization; descriptions and traits are size-limited and content
that resembles credentials or instruction-override payloads fails validation.

The schema contains no provider, credential, hidden instruction or execution fields.
Consumers must validate before persistence or composition. Precedence remains owned by
the instruction hierarchy contract; personality text is untrusted content.
