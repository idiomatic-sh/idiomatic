trait Parent {} trait Child: (Parent) {} struct Service; impl Parent for Service {}

trait Base {}
trait Contract: Base + Clone {}
impl Contract for module::Service {}
