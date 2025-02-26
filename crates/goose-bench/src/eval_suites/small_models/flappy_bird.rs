use crate::eval_suites::{BenchAgent, Evaluation, EvaluationMetric};
use crate::register_evaluation;
use crate::work_dir::WorkDir;
use async_trait::async_trait;

pub struct FlappyBird {}

impl FlappyBird {
    pub fn new() -> Self {
        FlappyBird {}
    }
}

#[async_trait]
impl Evaluation for FlappyBird {
    async fn run(
        &self,
        mut agent: Box<dyn BenchAgent>,
        _: &mut WorkDir,
    ) -> anyhow::Result<Vec<EvaluationMetric>> {
        println!("FlappyBird - run");
        let metrics = Vec::new();
        let _ = agent.prompt("Create a Flappy Bird game in Python in a file flappy_bird.py. You must include these things: You must use pygame. The background color should be randomly chosen and is a light shade. Start with a light blue color. Pressing SPACE multiple times will accelerate the bird. The bird's shape should be randomly chosen as a square, circle or triangle. The color should be randomly chosen as a dark color. Place on the bottom some land colored as dark brown or yellow chosen randomly. Make a score shown on the top right side. Increment if you pass pipes and don't hit them. Make randomly spaced pipes with enough space. Color them randomly as dark green or light brown or a dark gray shade. When you lose, show the best score. Make the text inside the screen. Pressing q or Esc will quit the game. Restarting is pressing SPACE again. Check your code for errors and fix them if any exist. The code should be in a flappy_bird.py file in python. Use the text_editor write tool.".to_string()).await;
        Ok(metrics)
    }

    fn name(&self) -> &str {
        "flappy_bird"
    }
}

register_evaluation!("small_models", FlappyBird);
