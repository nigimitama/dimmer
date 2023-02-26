const { PythonShell } = require("python-shell");
const path = require("path");

exports.getLuminance = async () => {
  // return: array[string]
  const scriptPath = path.join(__dirname, "python_modules/get_luminance.py");
  const result = await PythonShell.run(scriptPath);
  return result;
};

exports.setLuminance = async (value = null) => {
  const options = {
    args: [value],
  };
  const scriptPath = path.join(__dirname, "python_modules/set_luminance.py");
  return await PythonShell.run(scriptPath, options);
};
