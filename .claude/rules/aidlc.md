<!--
  .claude/rules/aidlc.md - the my-aidlc method @-import stub.

  my-aidlc runs the native AWS AI-DLC methodology - fetched from
  awslabs/aidlc-workflows at the version pinned in aidlc-version.json into the
  engine's native/ directory by `my-aidlc update` - across five phases:
  Initialization, Ideation, Inception, Construction, Operation. Invoke it as
  /aidlc.

  Workflow profiles live in the space at aidlc/spaces/<space>/scopes/ and are
  the team's to edit: each states its complete stage set in `include`.

  The method (org/team/project) is authored ONCE at the workspace root under
  aidlc/spaces/default/memory/. This file references it rather than copying it,
  so casual chat outside a workflow sees the standing practices. The engine's
  stage resolver reads the same tree directly and never needs this stub.

  Edit the method at aidlc/spaces/default/memory/*, never here.
-->

@../../aidlc/spaces/default/memory/org.md
@../../aidlc/spaces/default/memory/team.md
@../../aidlc/spaces/default/memory/project.md
