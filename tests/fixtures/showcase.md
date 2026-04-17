# Field Manual 42-B: Interdimensional Incident Response

## Preamble

This manual describes standard operating procedure for **Class-III
multiverse incursions**, as adopted by the *Intergalactic Committee on
Narrative Integrity* (ICNI). All personnel are required to read it twice,
sign the waiver, and memorize Section 4 before their next shift.

> "The universe is not only stranger than we imagine, it is stranger
> than we can be reimbursed for."
>
> — Admiral Ackbar, 2387 budget hearing
>
> > Accounting note: next time, please file form ***W-42B*** in triplicate.

---

## Section 1 — Crossover Classification

The following incursion types have been observed in the last
~~fiscal quarter~~ epoch:

| Class | Example                              | Threat Level |
|-------|--------------------------------------|--------------|
| Alpha | Gandalf at Comic-Con                 | Low          |
| Beta  | Spock attending a quidditch match    | Medium       |
| Gamma | A TARDIS inside a Death Star reactor | **CRITICAL** |

See [the full classification taxonomy](https://example.invalid/icni/taxonomy)
for additional detail, or browse the quick reference at
<https://example.invalid/icni/quick-ref>.

## Section 2 — Initial Response

Upon detecting an incursion, operatives should:

1. Confirm the breach using the `multiverse-scan --now` command.
2. Assess threat level per Section 1.
3. Notify your local liaison. **Do not** attempt to resolve Alpha-class
   incursions personally — they typically resolve themselves once coffee
   is offered.
4. File an incident report, even if the incident was "just a cat".

### Example Scan Output

```bash
$ multiverse-scan --now
[ok] scanning timelines 1..2^64
[warn] anomaly at (42, 7, 1977)
[critical] The One Ring detected inside a replicator unit
```

If the output looks garbled, try the unflagged variant:

```
multiverse-scan
```

Operators with code access prefer the Python wrapper, which returns
structured anomalies and mirrors the CLI flags:

```python
from multiverse import scanner

scan = scanner.scan(now=True, depth=64)
for anomaly in scan.anomalies:
    print(f"[{anomaly.severity}] {anomaly.description}")
```

## Section 3 — Field Kit

Operatives carry the following items at all times:

- A *sonic screwdriver* (standard issue)
- One (1) `phaser`, set to stun
- A small notebook labelled **Don't Panic**
- Emergency rations, consisting of:
  - Lembas bread
  - Replicator chits
  - A single chocolate frog, for morale

    Morale is critical on extended assignments. Personnel who neglect
    their chocolate frog ration have been observed to sulk, at which
    point rescue teams report a measurable drop in *snark throughput*.

The kit manifest is distributed as YAML so replicator units with only
minimal parsing capability can still honor a request:

```yaml
kit:
  standard:
    - sonic screwdriver
    - phaser
  morale:
    chocolate_frog: 1
    notebook: "Don't Panic"
```

### Escalation Chain

Incidents escalate in the following order:

1. Field operative
2. Shift supervisor
3. Regional coordinator
   1. First deputy
   2. Second deputy (Tuesdays only)
4. ICNI council

![Official ICNI seal](https://example.invalid/icni/seal.png)

---

## Appendix A — Poetry Corner

By long-standing tradition, every manual concludes with a verse. The
following was composed by R2-D2 during his tenure as guest editor:

Beep boop beep, the stars align,\
The Force and phasers intertwine,\
One does not simply walk to warp,\
So set your course, and mind the sharp.

*End of document.*
