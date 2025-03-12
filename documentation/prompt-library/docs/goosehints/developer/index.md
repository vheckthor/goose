---
sidebar_position: 1
---

# Developer GooseHints

<div className="prompt-grid">
  <div className="prompt-card">
    <h3>Project Analysis Hint</h3>
    <div>
      <span className="prompt-tag">Project Setup</span>
      <span className="prompt-tag">Analysis</span>
    </div>
    <p>Guide Goose in analyzing project structure</p>
    <pre>{`hints:
  project_analysis:
    context: "TypeScript project using React and Express"
    focus_areas:
      - code_organization
      - dependency_management`}</pre>
    <div className="prompt-meta">
      Extensions: Developer
    </div>
  </div>
  
  <div className="prompt-card">
    <h3>Code Generation Hint</h3>
    <div>
      <span className="prompt-tag">Code</span>
      <span className="prompt-tag">Generation</span>
    </div>
    <p>Set preferences for code generation style</p>
    <pre>{`hints:
  code_generation:
    style_guide: "airbnb"
    testing_framework: "jest"`}</pre>
    <div className="prompt-meta">
      Extensions: Developer
    </div>
  </div>
</div>