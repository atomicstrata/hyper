# External benchmark instrumentation; upstream production drawing is inherited.
extends "res://main.gd"

var samples: Array = []
var repetitions: Array = []
var frame_index = 0
var repetition = 0
var previous_usec = 0
var benchmark_camera: Camera2D
var center = Vector2.ZERO
var base_zoom = 1.0
var output_path = "res://benchmark_results.json"
var warmup_frames = 120
var measured_frames = 300
var repeat_count = 5
var startup_ms = 0.0
var exact_memberships = false

func _ready():
	seed(20261003)
	var started = Time.get_ticks_usec()
	var args = OS.get_cmdline_user_args()
	for index in range(args.size() - 1):
		match args[index]:
			"--output": output_path = args[index + 1]
			"--warmup": warmup_frames = int(args[index + 1])
			"--frames": measured_frames = int(args[index + 1])
			"--repetitions": repeat_count = int(args[index + 1])
	var graph = JSON.parse_string(FileAccess.get_file_as_string("res://benchmark_input.json"))
	var ids = JSON.parse_string(FileAccess.get_file_as_string("res://benchmark_idmap.json"))
	var data = read_file("res://patterns.txt")
	if data["nodes"].size() != graph["vertices"].size() or data["edges"].size() != graph["hyperedges"].size():
		push_error("Parser changed canonical node/group counts")
		get_tree().quit(3)
		return
	for i in range(graph["hyperedges"].size()):
		var expected: Array = []
		for member in graph["hyperedges"][i]["vertices"]:
			expected.append(ids[member])
		expected.sort()
		var actual: Array = Array(data["edges"][i]["nodes"])
		actual.sort()
		if actual != expected:
			push_error("Parser changed memberships for group " + str(i))
			get_tree().quit(3)
			return
	create_nodes(data["nodes"])
	populate_edge_data(data)
	if get_tree().get_nodes_in_group("edges").size() != graph["hyperedges"].size():
		push_error("Renderer omitted hyperedges")
		get_tree().quit(3)
		return
	exact_memberships = true
	var low = Vector2(INF, INF)
	var high = Vector2(-INF, -INF)
	for vertex in graph["vertices"]:
		var p = vertex["attrs"]["position"]
		var point = Vector2(float(p[0]), float(p[1]))
		get_node(ids[vertex["id"]]).position = point
		low = low.min(point)
		high = high.max(point)
	calculate_centrality_and_resize_nodes(data)
	for item in get_children():
		if item.has_method("mark_dirty"):
			item.mark_dirty()
	center = (low + high) * 0.5
	base_zoom = min(1920.0 / (high.x - low.x + 200.0), 1080.0 / (high.y - low.y + 200.0)) * 0.8
	benchmark_camera = Camera2D.new()
	add_child(benchmark_camera)
	benchmark_camera.position = center
	benchmark_camera.zoom = Vector2.ONE * base_zoom
	benchmark_camera.make_current()
	DisplayServer.window_set_size(Vector2i(1920, 1080))
	DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_WINDOWED)
	startup_ms = float(Time.get_ticks_usec() - started) / 1000.0
	previous_usec = Time.get_ticks_usec()

func _process(_delta):
	var now = Time.get_ticks_usec()
	if frame_index >= warmup_frames:
		samples.append(float(now - previous_usec) / 1000.0)
	previous_usec = now
	# Camera animation changes presentation without recomputing membership hulls.
	var t = float(frame_index) * 0.015
	benchmark_camera.position = center + Vector2(sin(t) * 30.0, cos(t) * 20.0)
	benchmark_camera.zoom = Vector2.ONE * base_zoom * (1.0 + sin(t * 0.4) * 0.08)
	frame_index += 1
	if samples.size() >= measured_frames:
		repetitions.append({"repetition": repetition, "frame_ms": samples.duplicate()})
		repetition += 1
		samples.clear()
		frame_index = 0
		if repetition >= repeat_count:
			finish_benchmark()

func finish_benchmark():
	var manifest = JSON.parse_string(FileAccess.get_file_as_string("res://benchmark_manifest.json"))
	manifest["godot_version"] = Engine.get_version_info()
	manifest["renderer"] = RenderingServer.get_current_rendering_method()
	manifest["renderer_driver"] = RenderingServer.get_current_rendering_driver_name()
	var window_size = DisplayServer.window_get_size()
	var screen_size = DisplayServer.screen_get_size()
	manifest["window_size"] = [window_size.x, window_size.y]
	manifest["screen_size"] = [screen_size.x, screen_size.y]
	manifest["window_mode"] = DisplayServer.window_get_mode()
	var viewport_size = get_viewport_rect().size
	manifest["viewport_size"] = [viewport_size.x, viewport_size.y]
	var render_size = get_viewport().get_texture().get_size()
	manifest["render_target_size"] = [render_size.x, render_size.y]
	manifest["matched_requested_viewport"] = render_size == Vector2(1920, 1080)
	manifest["adaptations"].append("stock HiDPI enabled; physical viewport verified")
	manifest["screen_scale"] = DisplayServer.screen_get_scale()
	manifest["screen_dpi"] = DisplayServer.screen_get_dpi()
	manifest["allow_hidpi"] = ProjectSettings.get_setting("display/window/dpi/allow_hidpi", true)
	manifest["warmup_frames"] = warmup_frames
	manifest["measured_frames"] = measured_frames
	manifest["startup_construction_ms"] = startup_ms
	manifest["exact_parser_memberships_and_rendered_group_count"] = exact_memberships
	manifest["color_seed"] = 20261003
	manifest["repetitions"] = repetitions
	var output = FileAccess.open(output_path, FileAccess.WRITE)
	if output == null:
		push_error("Cannot write benchmark output: " + output_path)
		get_tree().quit(2)
		return
	output.store_string(JSON.stringify(manifest, "\t"))
	output.close()
	get_tree().quit()
