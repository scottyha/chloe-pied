import type { ExtensionAPI, ExtensionContext } from "@mariozechner/pi-coding-agent";
import { withFileMutationQueue } from "@mariozechner/pi-coding-agent";
import { StringEnum } from "@mariozechner/pi-ai";
import { Type } from "typebox";
import * as crypto from "node:crypto";
import * as fs from "node:fs/promises";
import * as path from "node:path";

const STATE_DIRECTORY_NAME = ".chloe-pied";
const STATE_FILE_NAME = "state.json";
const DEFAULT_TASK_COLUMN = "Planning";
const DEFAULT_TASK_KIND = "Task";
const DEFAULT_ROADMAP_PRIORITY = "Medium";
const DEFAULT_ROADMAP_STATUS = "Planned";
const PROVIDER_NAME = "Pi";

type ChloeState = {
  tasks?: {
    columns?: Array<{
      name?: string;
      tasks?: unknown[];
    }>;
  };
  roadmap?: {
    items?: unknown[];
    selected_item?: number | null;
    mode?: string;
  };
};

type AddTaskParameters = {
  title: string;
  description: string;
  kind?: "Feature" | "Bug" | "Chore" | "Task";
  column?: string;
};

type AddRoadmapItemParameters = {
  title: string;
  description: string;
  rationale: string;
  priority?: "High" | "Medium" | "Low";
  user_stories?: string[];
  acceptance_criteria?: string[];
  tags?: string[];
};

async function pathExists(candidatePath: string): Promise<boolean> {
  try {
    await fs.access(candidatePath);
    return true;
  } catch {
    return false;
  }
}

async function findStatePath(context: ExtensionContext): Promise<string> {
  let currentDirectory = context.cwd;
  const rootDirectory = path.parse(currentDirectory).root;

  while (true) {
    const candidatePath = path.join(
      currentDirectory,
      STATE_DIRECTORY_NAME,
      STATE_FILE_NAME,
    );

    if (await pathExists(candidatePath)) {
      return candidatePath;
    }

    if (currentDirectory === rootDirectory) {
      break;
    }

    currentDirectory = path.dirname(currentDirectory);
  }

  throw new Error(
    "Could not find .chloe-pied/state.json in the current directory or any parent directory.",
  );
}

async function readState(statePath: string): Promise<ChloeState> {
  const stateData = await fs.readFile(statePath, "utf-8");
  return JSON.parse(stateData) as ChloeState;
}

async function writeState(statePath: string, state: ChloeState): Promise<void> {
  await fs.writeFile(statePath, `${JSON.stringify(state, null, 2)}\n`, "utf-8");
}

function createTask(parameters: AddTaskParameters) {
  return {
    id: crypto.randomUUID(),
    title: parameters.title,
    description: parameters.description,
    created_at: new Date().toISOString(),
    kind: parameters.kind ?? DEFAULT_TASK_KIND,
    provider: PROVIDER_NAME,
    instance_id: null,
    review_instance_id: null,
    is_paused: false,
    worktree_info: null,
  };
}

function addTaskToState(state: ChloeState, parameters: AddTaskParameters) {
  const targetColumnName = parameters.column ?? DEFAULT_TASK_COLUMN;
  const columns = state.tasks?.columns;

  if (!columns) {
    throw new Error("state.json does not contain tasks.columns.");
  }

  const targetColumn = columns.find((column) => column.name === targetColumnName);

  if (!targetColumn) {
    throw new Error(`Column '${targetColumnName}' not found in state.json.`);
  }

  if (!targetColumn.tasks) {
    targetColumn.tasks = [];
  }

  const task = createTask(parameters);
  targetColumn.tasks.push(task);

  return { task, targetColumnName };
}

function createRoadmapItem(parameters: AddRoadmapItemParameters) {
  const now = new Date().toISOString();

  return {
    id: crypto.randomUUID(),
    title: parameters.title,
    description: parameters.description,
    rationale: parameters.rationale,
    user_stories: parameters.user_stories ?? [],
    acceptance_criteria: parameters.acceptance_criteria ?? [],
    status: DEFAULT_ROADMAP_STATUS,
    priority: parameters.priority ?? DEFAULT_ROADMAP_PRIORITY,
    created_at: now,
    updated_at: now,
    dependencies: [],
    tags: parameters.tags ?? [],
  };
}

function addRoadmapItemToState(
  state: ChloeState,
  parameters: AddRoadmapItemParameters,
) {
  if (!state.roadmap) {
    state.roadmap = { items: [], selected_item: null, mode: "Normal" };
  }

  if (!state.roadmap.items) {
    state.roadmap.items = [];
  }

  const roadmapItem = createRoadmapItem(parameters);
  state.roadmap.items.push(roadmapItem);

  return roadmapItem;
}

export default function (pi: ExtensionAPI) {
  pi.registerTool({
    name: "add_chloe_task",
    label: "Add Chloe Task",
    description: "Add a new task directly to the Chloe-pied project board.",
    promptSnippet: "Add a task to the Chloe-pied project board",
    promptGuidelines: [
      "Use add_chloe_task when you need to create a new task in the Chloe-pied kanban board.",
    ],
    parameters: Type.Object({
      title: Type.String({ description: "Task title" }),
      description: Type.String({ description: "Task description" }),
      kind: Type.Optional(StringEnum(["Feature", "Bug", "Chore", "Task"] as const)),
      column: Type.Optional(
        Type.String({
          description: "Column to add the task to. Defaults to 'Planning'.",
        }),
      ),
    }),
    async execute(_toolCallId, parameters, _signal, _onUpdate, context) {
      const statePath = await findStatePath(context);

      return withFileMutationQueue(statePath, async () => {
        const state = await readState(statePath);
        const { task, targetColumnName } = addTaskToState(state, parameters);
        await writeState(statePath, state);

        return {
          content: [
            {
              type: "text" as const,
              text: `Added task '${parameters.title}' to the ${targetColumnName} column.`,
            },
          ],
          details: { taskId: task.id, statePath },
        };
      });
    },
  });

  pi.registerTool({
    name: "add_chloe_roadmap_item",
    label: "Add Chloe Roadmap Item",
    description: "Add a new strategic roadmap item to the Chloe-pied project.",
    promptSnippet: "Add a roadmap item to the Chloe-pied project board",
    promptGuidelines: [
      "Use add_chloe_roadmap_item to capture larger Chloe-pied features, epics, or strategic goals with rationale, user stories, and acceptance criteria.",
    ],
    parameters: Type.Object({
      title: Type.String({ description: "Roadmap item title" }),
      description: Type.String({
        description: "High-level description of the feature or goal",
      }),
      rationale: Type.String({
        description: "Why this matters or the business value it creates",
      }),
      priority: Type.Optional(StringEnum(["High", "Medium", "Low"] as const)),
      user_stories: Type.Optional(
        Type.Array(Type.String(), {
          description: "List of user stories",
        }),
      ),
      acceptance_criteria: Type.Optional(
        Type.Array(Type.String(), {
          description: "List of conditions that must be met for completion",
        }),
      ),
      tags: Type.Optional(
        Type.Array(Type.String(), { description: "Categorization tags" }),
      ),
    }),
    async execute(_toolCallId, parameters, _signal, _onUpdate, context) {
      const statePath = await findStatePath(context);

      return withFileMutationQueue(statePath, async () => {
        const state = await readState(statePath);
        const roadmapItem = addRoadmapItemToState(state, parameters);
        await writeState(statePath, state);

        return {
          content: [
            {
              type: "text" as const,
              text: `Added roadmap item '${parameters.title}'.`,
            },
          ],
          details: { roadmapItemId: roadmapItem.id, statePath },
        };
      });
    },
  });
}
