"use strict";

const vscode = require("vscode");
const cp = require("child_process");
const path = require("path");

function activate(context) {
  const output = vscode.window.createOutputChannel("Void Memingo");
  context.subscriptions.push(output);

  const command = vscode.commands.registerCommand("voidMemingo.runFile", async () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== "void") {
      vscode.window.showWarningMessage("Open a .void file first.");
      return;
    }

    await editor.document.save();
    const file = editor.document.fileName;
    output.clear();
    output.show(true);
    output.appendLine("> void run " + file);

    const child = cp.spawn("void", ["run", file], {
      cwd: path.dirname(file),
      shell: process.platform === "win32"
    });

    child.stdout.on("data", data => output.append(data.toString()));
    child.stderr.on("data", data => output.append(data.toString()));
    child.on("error", error => {
      output.appendLine("\nCould not start 'void'. Install Void Memingo and ensure it is on PATH.");
      output.appendLine(error.message);
    });
    child.on("close", code => output.appendLine("\nProcess exited with code " + code));
  });

  context.subscriptions.push(command);
}

function deactivate() {}

module.exports = { activate, deactivate };
