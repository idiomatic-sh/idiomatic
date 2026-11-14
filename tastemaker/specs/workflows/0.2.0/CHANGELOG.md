# workflows

## 0.2.0

The descriptions state the `ci` spec's 0.2.0 triggers: a workflow starts by hand, and may also start on a pull request to main or a push to main, as `ci/workflows_test.py` allows (0.1.0 said its one trigger was `workflow_dispatch`). The content is unchanged: a workflow still names itself and `workflow_dispatch`.

### Moving up from 0.1.0

Nothing changes in a workflow.

## 0.1.0

The workflows under `.github/workflows/`, moved out of the `ci` spec when the repository root's `tastemaker.toml` opened: it places the `thin-workflow` template as `ci` did, applied at `.github/workflows`, and is closed, so a script among the workflows is a finding.
