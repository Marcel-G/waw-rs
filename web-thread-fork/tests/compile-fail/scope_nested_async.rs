//@only-target-wasm32-unknown-unknown

fn test() {
	web_thread::web::scope_async(|scope| async {
		scope.spawn(|| {
			let mut test = 0;

			scope.spawn(|| test = 1);
            //~^ ERROR: closure may outlive the current function, but it borrows `test`, which is owned by the current function
		});
	});
}
