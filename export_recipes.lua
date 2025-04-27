--[[
	Generates recipes.json from in-game API.
	To run it:
	1. Open the in-game console (default key is ~).
	2. Input
		/c game.player.print(1)
	to test it (you may need to do it twice to enable cheats).
	3. Input
		/c
	and paste this entire file from clipboard. Press Enter to execute it.
--]]

--[[
	properties are listed at https://lua-api.factorio.com/latest/classes/LuaRecipe.html
	"group", "subgroup", "force" are skipped because they are always null;
	"valid" is skipped because it's always true.
--]]
local recipe_properties = {
	"name",
	"enabled",
	"category",
	"ingredients",
	"products",
	"hidden",
	"hidden_from_flow_stats",
	"energy",
	"order",
	"productivity_bonus",
}
--[[ serialization doesn't work on custom lua values, so we'll convert them to tables ]]
local recipes_table = {}
local counter = 0
for k, recipe in pairs(game.player.force.recipes) do
  local recipe_table = {}
  for _, prop in pairs(recipe_properties) do
	recipe_table[prop] = recipe[prop]
  end
  recipes_table[k] = recipe_table
  counter = counter + 1
end
helpers.write_file("recipes.json", helpers.table_to_json(recipes_table))
game.player.print("Exported "..counter.." recipes to %appdata%\\Factorio\\script-output\\recipes.json")
