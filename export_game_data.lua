--[[
	Generates game_data.json from in-game API.
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
local num_recipes = 0
for k, recipe in pairs(game.player.force.recipes) do
  local recipe_table = {}
  for _, prop in pairs(recipe_properties) do
	recipe_table[prop] = recipe[prop]
  end
  recipes_table[k] = recipe_table
  num_recipes = num_recipes + 1
end

--[[ https://lua-api.factorio.com/latest/classes/LuaEntityPrototype.html ]]
local entity_properties = {
	"type",
	"name",
	"energy_usage",
	--[[ CraftingMachine ]]
	"crafting_categories",
	"ingredient_count",
	"max_item_product_count",
	--[[ MiningDrill ]]
	"mining_speed",
	"resource_categories",
	--[[ TransportBeltConnectable ]]
	--[[ tiles per tick; throughput per second = belt_speed * 60 (ticks/s) * 8 (density) ]]
	"belt_speed",
	--[[ ResourceEntity ]]
	"resource_category",
	--[[ OffshorePump ]]
	"pumping_speed",
}
local entities_table = {}
local num_entities = 0
for k, entity in pairs(prototypes.entity) do
	if entity.crafting_categories or
		entity.mining_speed or
		entity.pumping_speed or
		entity.type == "transport-belt"
		or entity.type == "resource"
		or entity.type == "plant"
		or entity.type == "tree"
	then
		local entity_table = {}
		for _, prop in pairs(entity_properties) do
			entity_table[prop] = entity[prop]
		end
		if entity.crafting_categories then
			entity_table["crafting_speed"] = entity.get_crafting_speed()
		end
		if entity.type == "resource" or entity.type == "plant" or entity.type == "tree"  then
			entity_table.mineable_properties = {
				mining_time = entity.mineable_properties.mining_time,
				products = entity.mineable_properties.products,
				fluid_amount = entity.mineable_properties.fluid_amount,
				required_fluid = entity.mineable_properties.required_fluid,
			}
		end
		entities_table[k] = entity_table
		num_entities = num_entities + 1
	end
end

local data = {
	recipes = recipes_table,
	entities = entities_table,
}

helpers.write_file("game_data.json", helpers.table_to_json(data))
game.player.print("Exported "..num_recipes.." recipes and "..num_entities.." entities to %appdata%\\Factorio\\script-output\\game_data.json")
