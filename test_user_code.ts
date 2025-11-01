export class CentralUsersModule extends HttpFactory {
  protected $events = {}

  async checkAuthenticated() {
    return await this.call('get', '/check')
  }

  async getAllUsers(params) {
    return await this.call('get', `/users`, params)
  }

  async deleteUser(userId) {
    return await this.call('delete', `/user/${userId}`)
  }
}
