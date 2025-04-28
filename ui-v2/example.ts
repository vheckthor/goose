// add necessary imports from src/a2a

const client = new A2AClient("http://localhost:41241");

// Send a task
const taskResult = await client.sendTask({
  id: uuidv4(),
  message: {
    role: "user",
    parts: [{ text: "Hello, agent!", type: "text" }]
  }
});

// Check task status
const task = await client.getTask({ id: taskResult.id });
