fn main(){
	tonic_prost_build::compile_protos(
		"../proto_library/users.proto"
	).unwrap();

	tonic_prost_build::compile_protos(
		"../proto_library/chat.proto"
	).unwrap();

}