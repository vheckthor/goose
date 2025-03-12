---
sidebar_position: 2
---

# Business GooseHints

<div className="prompt-grid">
  <div className="prompt-card">
    <h3>Document Processing Hint</h3>
    <div>
      <span className="prompt-tag">Documents</span>
      <span className="prompt-tag">Processing</span>
    </div>
    <p>Configure Goose for handling business documents</p>
    <pre>{`hints:
  document_processing:
    document_types:
      - reports
      - presentations
    extraction_preferences:
      - metrics
      - action_items`}</pre>
    <div className="prompt-meta">
      Extensions: ComputerController
    </div>
  </div>
  
  <div className="prompt-card">
    <h3>Workflow Automation Hint</h3>
    <div>
      <span className="prompt-tag">Workflow</span>
      <span className="prompt-tag">Automation</span>
    </div>
    <p>Define preferences for automated workflows</p>
    <pre>{`hints:
  workflow_automation:
    notification_preferences:
      - email
      - slack
    reporting_format: "pdf"`}</pre>
    <div className="prompt-meta">
      Extensions: ComputerController
    </div>
  </div>
</div>