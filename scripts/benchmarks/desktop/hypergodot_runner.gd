extends SceneTree

func _initialize():
	var scene = load("res://main.tscn").instantiate()
	scene.set_script(load("res://benchmark_scene.gd"))
	root.add_child(scene)
